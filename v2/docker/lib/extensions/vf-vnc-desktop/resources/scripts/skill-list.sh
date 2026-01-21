#!/bin/bash
# List all available Claude Code skills

SKILLS_DIR="/home/devuser/.claude/skills"

echo "=== Available Claude Code Skills ==="
echo ""

if [ ! -d "$SKILLS_DIR" ]; then
    echo "❌ Skills directory not found: $SKILLS_DIR"
    exit 1
fi

skill_count=0

for skill_dir in "$SKILLS_DIR"/*; do
    if [ -d "$skill_dir" ] && [ -f "$skill_dir/SKILL.md" ]; then
        skill_name=$(basename "$skill_dir")

        # Extract name and description from YAML frontmatter
        name=$(grep "^name:" "$skill_dir/SKILL.md" | cut -d':' -f2- | xargs)
        description=$(grep "^description:" "$skill_dir/SKILL.md" | cut -d':' -f2- | xargs)

        echo "📦 $skill_name"
        echo "   Name: $name"
        echo "   Description: $description"
        echo ""

        ((skill_count++))
    fi
done

echo "Total: $skill_count skills"
