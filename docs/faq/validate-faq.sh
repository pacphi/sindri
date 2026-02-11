#!/bin/bash
set -e

FAQ_FILE="docs/faq/src/v3-faq-data.json"

echo "Validating FAQ data schema..."

# Validate JSON syntax
if ! jq empty "$FAQ_FILE" 2>/dev/null; then
  echo "❌ Invalid JSON syntax"
  exit 1
fi

# Check metadata count
META_COUNT=$(jq '.meta.totalQuestions' "$FAQ_FILE")
ACTUAL_COUNT=$(jq '.questions | length' "$FAQ_FILE")

if [ "$META_COUNT" != "$ACTUAL_COUNT" ]; then
  echo "❌ Metadata mismatch: meta says $META_COUNT, actual count is $ACTUAL_COUNT"
  exit 1
else
  echo "✅ Question count: $ACTUAL_COUNT"
fi

# Check for duplicate question IDs
DUPLICATES=$(jq -r '.questions[].id' "$FAQ_FILE" | sort | uniq -d)
if [ -n "$DUPLICATES" ]; then
  echo "❌ Duplicate question IDs found:"
  echo "$DUPLICATES"
  exit 1
else
  echo "✅ All question IDs unique"
fi

# Validate category references
INVALID_CATS=$(comm -23 \
  <(jq -r '.questions[].category' "$FAQ_FILE" | sort -u) \
  <(jq -r '.categories[].id' "$FAQ_FILE" | sort))

if [ -n "$INVALID_CATS" ]; then
  echo "❌ Invalid category references:"
  echo "$INVALID_CATS"
  exit 1
else
  echo "✅ All category references valid"
fi

# Validate persona references
INVALID_PERSONAS=$(comm -23 \
  <(jq -r '.questions[].personas[]' "$FAQ_FILE" | sort -u) \
  <(jq -r '.personas[].id' "$FAQ_FILE" | sort))

if [ -n "$INVALID_PERSONAS" ]; then
  echo "❌ Invalid persona references:"
  echo "$INVALID_PERSONAS"
  exit 1
else
  echo "✅ All persona references valid"
fi

# Validate use case references
INVALID_USECASES=$(comm -23 \
  <(jq -r '.questions[].useCases[]' "$FAQ_FILE" | sort -u) \
  <(jq -r '.useCases[].id' "$FAQ_FILE" | sort))

if [ -n "$INVALID_USECASES" ]; then
  echo "❌ Invalid use case references:"
  echo "$INVALID_USECASES"
  exit 1
else
  echo "✅ All use case references valid"
fi

# Check for questions without version tags
NO_VERSION_TAGS=$(jq -r '.questions[] | select(.tags | any(test("v2|v3|migration")) | not) | .id' "$FAQ_FILE")

if [ -n "$NO_VERSION_TAGS" ]; then
  echo "⚠️  Questions without version tags:"
  echo "$NO_VERSION_TAGS"
else
  echo "✅ All questions have version tags"
fi

# Check for questions exceeding tag limit
EXCESSIVE_TAGS=$(jq '.questions[] | select(.tags | length > 8) | {id, count: (.tags | length)}' "$FAQ_FILE")

if [ -n "$EXCESSIVE_TAGS" ]; then
  echo "⚠️  Questions exceeding 8 tag limit:"
  echo "$EXCESSIVE_TAGS"
else
  echo "✅ All questions within tag limit"
fi

echo ""
echo "✅ Validation complete!"
echo ""
echo "Statistics:"
echo "  Questions: $ACTUAL_COUNT"
echo "  Categories: $(jq '.categories | length' "$FAQ_FILE")"
echo "  Personas: $(jq '.personas | length' "$FAQ_FILE")"
echo "  Use Cases: $(jq '.useCases | length' "$FAQ_FILE")"
echo "  v2-only: $(jq '[.questions[] | select(.versionsApplicable == ["v2"])] | length' "$FAQ_FILE")"
echo "  v3-only: $(jq '[.questions[] | select(.versionsApplicable == ["v3"])] | length' "$FAQ_FILE")"
echo "  Shared: $(jq '[.questions[] | select(.versionsApplicable == ["v2", "v3"] or .versionsApplicable == ["v3", "v2"])] | length' "$FAQ_FILE")"
