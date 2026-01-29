#!/usr/bin/env bash
# Migration Guide Generation Script for Sindri
# Usage: ./generate-migration-guide.sh <from-version> <to-version> <output-file>
#
# Examples:
#   ./generate-migration-guide.sh v2.2.1 v3.0.0 RELEASE_NOTES.v3.md
#   ./generate-migration-guide.sh v3.0.0 v3.1.0 MIGRATION_v3.1.md
#
# Extracts breaking changes from conventional commits:
#   - BREAKING CHANGE: footer
#   - feat! / fix! / refactor! (breaking change indicator)
#   - Generates draft migration guide using template

set -euo pipefail

if [[ $# -lt 3 ]]; then
  echo "Usage: $0 <from-version> <to-version> <output-file>" >&2
  echo "Example: $0 v2.2.1 v3.0.0 RELEASE_NOTES.v3.md" >&2
  exit 1
fi

FROM_VERSION="$1"
TO_VERSION="$2"
OUTPUT_FILE="$3"
TEMPLATE_FILE="${TEMPLATE_FILE:-.github/templates/release-notes-template.md}"

echo "Generating migration guide: $FROM_VERSION → $TO_VERSION" >&2

# Extract version prefix (v2, v3, etc.)
TO_PREFIX=$(echo "$TO_VERSION" | grep -oP '^v\d+')
FROM_PREFIX=$(echo "$FROM_VERSION" | grep -oP '^v\d+')

# Determine path filter based on target version
if [[ "$TO_PREFIX" == "v3" ]]; then
  PATH_FILTER="v3/"
elif [[ "$TO_PREFIX" == "v2" ]]; then
  PATH_FILTER="v2/"
else
  PATH_FILTER="."
fi

echo "Path filter: $PATH_FILTER" >&2

# Collect breaking changes
breaking_changes=()
breaking_commits=()

# Find commits with BREAKING CHANGE: footer or ! indicator
while IFS= read -r commit_hash; do
  [[ -z "$commit_hash" ]] && continue

  # Get full commit message
  full_message=$(git log -1 --format=%B "$commit_hash")
  subject=$(git log -1 --format=%s "$commit_hash")

  # Check for breaking change indicators
  is_breaking=false
  breaking_reason=""

  # Method 1: ! in commit type (e.g., feat!:, fix!:)
  if [[ "$subject" =~ ^[a-z]+(\(.+\))?!: ]]; then
    is_breaking=true
    breaking_reason="Breaking change indicator (!) in commit message"
  fi

  # Method 2: BREAKING CHANGE: in footer
  if echo "$full_message" | grep -qi "^BREAKING CHANGE:"; then
    is_breaking=true
    breaking_reason="BREAKING CHANGE footer found"
  fi

  # Method 3: BREAKING-CHANGE: variant
  if echo "$full_message" | grep -qi "^BREAKING-CHANGE:"; then
    is_breaking=true
    breaking_reason="BREAKING-CHANGE footer found"
  fi

  if [[ "$is_breaking" == "true" ]]; then
    breaking_changes+=("$subject")
    breaking_commits+=("$commit_hash")
    echo "  Found: $subject ($commit_hash)" >&2
  fi
done < <(git log --format=%H "$FROM_VERSION..$TO_VERSION" -- "$PATH_FILTER" 2>/dev/null || git log --format=%H "$FROM_VERSION..$TO_VERSION")

echo "" >&2
echo "Found ${#breaking_changes[@]} breaking change(s)" >&2

# Read template if it exists, otherwise use basic template
if [[ -f "$TEMPLATE_FILE" ]]; then
  echo "Using template: $TEMPLATE_FILE" >&2
  template_content=$(<"$TEMPLATE_FILE")
else
  echo "Template not found, using basic template" >&2
  template_content="# Migration Guide: $FROM_VERSION → $TO_VERSION

## Breaking Changes

{{BREAKING_CHANGES}}

## Migration Steps

{{MIGRATION_STEPS}}

## What's New

See the [CHANGELOG](CHANGELOG.md) for a complete list of changes.

## Need Help?

- **Issues**: Report problems at https://github.com/\${GITHUB_REPOSITORY}/issues
- **Discussions**: Ask questions at https://github.com/\${GITHUB_REPOSITORY}/discussions
"
fi

# Build breaking changes section
breaking_section=""
if [[ ${#breaking_changes[@]} -eq 0 ]]; then
  breaking_section="No breaking changes identified in conventional commits.

> **Note**: This section was auto-generated. Manual review recommended to ensure no breaking changes were missed."
else
  breaking_section+="The following breaking changes were detected:"$'\n\n'
  for i in "${!breaking_changes[@]}"; do
    change="${breaking_changes[$i]}"
    commit="${breaking_commits[$i]}"
    breaking_section+="### $((i+1)). ${change}"$'\n\n'
    breaking_section+="**Commit**: \`${commit}\`"$'\n\n'

    # Extract BREAKING CHANGE description from commit body if available
    breaking_desc=$(git log -1 --format=%B "$commit" | sed -n '/^BREAKING[ -]CHANGE:/,/^$/p' | tail -n +2 | sed 's/^[[:space:]]*//')
    if [[ -n "$breaking_desc" ]]; then
      breaking_section+="**Description**:"$'\n'
      breaking_section+="$breaking_desc"$'\n\n'
    fi

    breaking_section+="**TODO**: Add migration instructions, code examples (before/after), and user impact."$'\n\n'
    breaking_section+="---"$'\n\n'
  done
fi

# Build migration steps section (placeholder for manual enrichment)
migration_steps="## Migration Checklist

### For Extension Authors

- [ ] Review breaking changes above
- [ ] Update extension code for API changes
- [ ] Test extension with $TO_VERSION
- [ ] Update extension documentation
- [ ] Update \`extension.yaml\` if needed

### For End Users

- [ ] Review breaking changes above
- [ ] Backup existing configuration
- [ ] Update Docker images or binaries
- [ ] Test workflows in non-production environment
- [ ] Update CI/CD pipelines if needed

### For DevOps/Platform Teams

- [ ] Review infrastructure requirements
- [ ] Update deployment manifests
- [ ] Test rollout in staging environment
- [ ] Plan rollback strategy
- [ ] Update monitoring/alerting for new version

> **Note**: This checklist is a starting point. Customize based on your use case.
"

# Replace placeholders
output_content="${template_content//\{\{BREAKING_CHANGES\}\}/$breaking_section}"
output_content="${output_content//\{\{MIGRATION_STEPS\}\}/$migration_steps}"
output_content="${output_content//\{\{FROM_VERSION\}\}/$FROM_VERSION}"
output_content="${output_content//\{\{TO_VERSION\}\}/$TO_VERSION}"

# Write to output file
echo "$output_content" > "$OUTPUT_FILE"

echo "" >&2
echo "✅ Migration guide draft written to: $OUTPUT_FILE" >&2
echo "" >&2
echo "Next steps:" >&2
echo "  1. Review auto-extracted breaking changes" >&2
echo "  2. Add detailed migration instructions" >&2
echo "  3. Include code examples (before/after)" >&2
echo "  4. Add troubleshooting section" >&2
echo "  5. Test migration guide with real deployments" >&2
