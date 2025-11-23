#!/usr/bin/env bash
# Generate Slack notification message for manual deployment
# Usage: generate-slack-notification.sh <status> <app-name> <provider> <environment> <url> <actor> <run-id>

set -euo pipefail

STATUS="${1:-}"
APP_NAME="${2:-}"
PROVIDER="${3:-}"
ENVIRONMENT="${4:-}"
URL="${5:-}"
ACTOR="${6:-}"
RUN_ID="${7:-}"

if [[ "$STATUS" == "success" ]]; then
  STATUS_EMOJI="✅"
  STATUS_TEXT="successful"
else
  STATUS_EMOJI="❌"
  STATUS_TEXT="failed"
fi

cat << EOF
{
  "text": "Manual Deployment ${STATUS_TEXT}",
  "blocks": [
    {
      "type": "section",
      "text": {
        "type": "mrkdwn",
        "text": "${STATUS_EMOJI} *Manual Deployment ${STATUS_TEXT}*\\n*App:* ${APP_NAME}\\n*Provider:* ${PROVIDER}\\n*Environment:* ${ENVIRONMENT}\\n*URL:* ${URL}"
      }
    },
    {
      "type": "context",
      "elements": [
        {
          "type": "mrkdwn",
          "text": "Deployed by: ${ACTOR} | Run: ${RUN_ID}"
        }
      ]
    }
  ]
}
EOF
