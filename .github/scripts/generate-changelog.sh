#!/usr/bin/env bash
# Consolidated Changelog Generation Script for Sindri v2/v3
# Usage: ./generate-changelog.sh <version> <version-prefix> <path-filter> <output-file>
#
# Examples:
#   ./generate-changelog.sh 2.3.0 v2 v2/ v2/CHANGELOG-new.md
#   ./generate-changelog.sh 3.1.0 v3 v3/ v3/CHANGELOG-new.md
#
# Arguments:
#   version        - Version number without prefix (e.g., 2.3.0)
#   version-prefix - Tag prefix (v2 or v3)
#   path-filter    - Git path filter (v2/ or v3/)
#   output-file    - Output file path

set -euo pipefail

# Check arguments
if [[ $# -lt 3 ]]; then
  echo "Usage: $0 <version> <version-prefix> <path-filter> [output-file]" >&2
  echo "Example: $0 2.3.0 v2 v2/ changelog.md" >&2
  exit 1
fi

VERSION="$1"
VERSION_PREFIX="$2"
PATH_FILTER="$3"
OUTPUT_FILE="${4:-changelog.md}"

# Construct current tag - VERSION already contains the full semver (e.g., 3.0.0-alpha.8)
# so we just need to prepend "v"
CURRENT_TAG="v${VERSION}"
REPO="${GITHUB_REPOSITORY:-unknown/repo}"

echo "Generating changelog for $CURRENT_TAG (path filter: $PATH_FILTER)" >&2

# Get previous tag for this version prefix
# List all tags, find current tag's position, get the next one (previous release)
ALL_TAGS=$(git tag -l "${VERSION_PREFIX}.*" --sort=-version:refname)
PREVIOUS_TAG=""

found_current=false
while IFS= read -r tag; do
  if [[ "$found_current" == "true" ]]; then
    PREVIOUS_TAG="$tag"
    break
  fi
  if [[ "$tag" == "$CURRENT_TAG" ]]; then
    found_current=true
  fi
done <<< "$ALL_TAGS"

if [[ -z "$PREVIOUS_TAG" ]]; then
  echo "No previous ${VERSION_PREFIX} tag found, checking for first commit" >&2
  # Get the first commit that touches this path as the base
  # Note: Use head in a way that avoids SIGPIPE errors with pipefail
  FIRST_COMMIT=$(git rev-list --reverse HEAD -- "$PATH_FILTER" 2>/dev/null | head -1) || true
  if [[ -n "$FIRST_COMMIT" ]]; then
    COMMIT_RANGE="${FIRST_COMMIT}^..$CURRENT_TAG"
    echo "Using commits from first v3 commit to $CURRENT_TAG" >&2
  else
    COMMIT_RANGE="$CURRENT_TAG"
  fi
else
  echo "Generating changelog from $PREVIOUS_TAG to $CURRENT_TAG" >&2
  COMMIT_RANGE="$PREVIOUS_TAG..$CURRENT_TAG"
fi

# Initialize changelog sections
features=""
fixes=""
docs=""
deps=""
perf=""
refactor=""
chore=""
tests=""
other=""

# Parse commits filtered by path
while IFS= read -r commit; do
  [[ -z "$commit" ]] && continue

  # Extract commit hash and message
  hash="${commit:0:7}"
  message="${commit:8}"

  # Categorize by conventional commit prefix
  case "$message" in
    feat:*|feat\(*)       features+="- $message ($hash)"$'\n' ;;
    fix:*|fix\(*)         fixes+="- $message ($hash)"$'\n' ;;
    docs:*|docs\(*)       docs+="- $message ($hash)"$'\n' ;;
    deps:*|deps\(*)       deps+="- $message ($hash)"$'\n' ;;
    perf:*|perf\(*)       perf+="- $message ($hash)"$'\n' ;;
    refactor:*|refactor\(*) refactor+="- $message ($hash)"$'\n' ;;
    chore:*|chore\(*)     chore+="- $message ($hash)"$'\n' ;;
    test:*|test\(*)       tests+="- $message ($hash)"$'\n' ;;
    ci:*|ci\(*)           chore+="- $message ($hash)"$'\n' ;;
    style:*|style\(*)     chore+="- $message ($hash)"$'\n' ;;
    *)                    other+="- $message ($hash)"$'\n' ;;
  esac
done < <(git log --oneline "$COMMIT_RANGE" -- "$PATH_FILTER" 2>/dev/null || git log --oneline -- "$PATH_FILTER")

# Build changelog content
changelog="## [${VERSION}] - $(date +%Y-%m-%d)"$'\n\n'

[[ -n "$features" ]] && changelog+="### :sparkles: Features"$'\n\n'"$features"$'\n'
[[ -n "$fixes" ]] && changelog+="### :bug: Bug Fixes"$'\n\n'"$fixes"$'\n'
[[ -n "$docs" ]] && changelog+="### :books: Documentation"$'\n\n'"$docs"$'\n'
[[ -n "$deps" ]] && changelog+="### :package: Dependencies"$'\n\n'"$deps"$'\n'
[[ -n "$perf" ]] && changelog+="### :zap: Performance"$'\n\n'"$perf"$'\n'
[[ -n "$refactor" ]] && changelog+="### :recycle: Refactoring"$'\n\n'"$refactor"$'\n'
[[ -n "$tests" ]] && changelog+="### :white_check_mark: Tests"$'\n\n'"$tests"$'\n'
[[ -n "$chore" ]] && changelog+="### :wrench: Maintenance"$'\n\n'"$chore"$'\n'
[[ -n "$other" ]] && changelog+="### :gear: Other Changes"$'\n\n'"$other"$'\n'

# Add version-specific installation instructions
changelog+="### Installation"$'\n\n'

if [[ "$VERSION_PREFIX" == "v2" ]]; then
  # v2 Docker-only installation
  changelog+='```bash'$'\n'
  changelog+="# Pull Docker image"$'\n'
  changelog+="docker pull ghcr.io/${REPO}:v${VERSION}"$'\n\n'
  changelog+="# Or use specific tag"$'\n'
  changelog+="docker pull ghcr.io/${REPO}:v2"$'\n\n'
  changelog+="# Run container"$'\n'
  changelog+="docker run -it -v sindri-home:/alt/home/developer ghcr.io/${REPO}:v${VERSION}"$'\n'
  changelog+='```'$'\n\n'
elif [[ "$VERSION_PREFIX" == "v3" ]]; then
  # v3 Docker + binary installation
  changelog+="#### Docker Image"$'\n'
  changelog+='```bash'$'\n'
  changelog+="# Pull the Docker image"$'\n'
  changelog+="docker pull ghcr.io/${REPO}:${VERSION}"$'\n\n'
  changelog+="# Run a container"$'\n'
  changelog+="docker run -d --name sindri \\"$'\n'
  changelog+="  -e SINDRI_PROFILE=minimal \\"$'\n'
  changelog+="  -v sindri_home:/alt/home/developer \\"$'\n'
  changelog+="  ghcr.io/${REPO}:${VERSION}"$'\n'
  changelog+='```'$'\n\n'
  changelog+="#### CLI Binary"$'\n'
  changelog+='```bash'$'\n'
  changelog+="# Download and install from release assets"$'\n'
  changelog+="# Linux (x86_64)"$'\n'
  changelog+="wget https://github.com/${REPO}/releases/download/v${VERSION}/sindri-v${VERSION}-linux-x86_64.tar.gz"$'\n'
  changelog+="tar -xzf sindri-v${VERSION}-linux-x86_64.tar.gz"$'\n'
  changelog+="sudo mv sindri /usr/local/bin/"$'\n\n'
  changelog+="# macOS (Apple Silicon)"$'\n'
  changelog+="wget https://github.com/${REPO}/releases/download/v${VERSION}/sindri-v${VERSION}-macos-aarch64.tar.gz"$'\n'
  changelog+="tar -xzf sindri-v${VERSION}-macos-aarch64.tar.gz"$'\n'
  changelog+="sudo mv sindri /usr/local/bin/"$'\n'
  changelog+='```'$'\n\n'
fi

# Add diff link if previous tag exists
if [[ -n "$PREVIOUS_TAG" ]] && [[ "$PREVIOUS_TAG" != "$CURRENT_TAG" ]]; then
  changelog+="**Full Changelog**: https://github.com/${REPO}/compare/$PREVIOUS_TAG...$CURRENT_TAG"$'\n'
fi

# Write to output file
echo "$changelog" > "$OUTPUT_FILE"

echo "Changelog written to $OUTPUT_FILE" >&2
echo "Changelog for ${CURRENT_TAG} generated successfully!" >&2
