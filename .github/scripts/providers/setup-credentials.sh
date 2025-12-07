#!/usr/bin/env bash
# setup-credentials.sh - Unified credential setup for all providers
# Usage: source setup-credentials.sh <provider>
# Sets up environment and credentials for provider-specific testing

set -euo pipefail

PROVIDER="${1:?Provider required}"

echo "Setting up credentials for provider: $PROVIDER"

case "$PROVIDER" in
    docker)
        echo "✓ No credentials required for Docker provider"
        ;;

    fly)
        if [[ -z "${FLY_API_TOKEN:-}" ]]; then
            echo "::error::FLY_API_TOKEN is required for fly provider"
            exit 1
        fi
        echo "✓ Fly.io credentials configured"
        # flyctl reads FLY_API_TOKEN from environment
        ;;

    devpod-aws)
        if [[ -z "${AWS_ACCESS_KEY_ID:-}" ]] || [[ -z "${AWS_SECRET_ACCESS_KEY:-}" ]]; then
            echo "::error::AWS credentials required for devpod-aws provider"
            exit 1
        fi
        echo "✓ AWS credentials configured"
        # AWS CLI reads from environment variables
        ;;

    devpod-gcp)
        if [[ -z "${GCP_SERVICE_ACCOUNT_KEY:-}" ]]; then
            echo "::error::GCP_SERVICE_ACCOUNT_KEY is required for devpod-gcp provider"
            exit 1
        fi

        # Write GCP credentials to file
        mkdir -p ~/.config/gcloud
        echo "$GCP_SERVICE_ACCOUNT_KEY" > ~/.config/gcloud/application_default_credentials.json
        chmod 600 ~/.config/gcloud/application_default_credentials.json

        export GOOGLE_APPLICATION_CREDENTIALS=~/.config/gcloud/application_default_credentials.json
        echo "GOOGLE_APPLICATION_CREDENTIALS=$GOOGLE_APPLICATION_CREDENTIALS" >> "${GITHUB_ENV:-/dev/null}"

        echo "✓ GCP credentials configured"
        ;;

    devpod-azure)
        if [[ -z "${AZURE_CLIENT_ID:-}" ]] || [[ -z "${AZURE_CLIENT_SECRET:-}" ]] || [[ -z "${AZURE_TENANT_ID:-}" ]]; then
            echo "::error::Azure credentials required for devpod-azure provider"
            exit 1
        fi
        echo "✓ Azure credentials configured"
        # Azure CLI reads from environment variables
        ;;

    devpod-do)
        if [[ -z "${DIGITALOCEAN_TOKEN:-}" ]]; then
            echo "::error::DIGITALOCEAN_TOKEN is required for devpod-do provider"
            exit 1
        fi
        echo "✓ DigitalOcean credentials configured"
        # DevPod reads token from environment
        ;;

    devpod-k8s|kubernetes)
        # KUBECONFIG can be file content or path
        if [[ -n "${KUBECONFIG:-}" ]]; then
            # Check if KUBECONFIG is a file path or content
            if [[ -f "$KUBECONFIG" ]]; then
                echo "✓ Using existing kubeconfig at $KUBECONFIG"
            else
                # Assume it's file content, write it
                mkdir -p ~/.kube
                echo "$KUBECONFIG" > ~/.kube/config
                chmod 600 ~/.kube/config
                export KUBECONFIG=~/.kube/config
                echo "KUBECONFIG=$KUBECONFIG" >> "${GITHUB_ENV:-/dev/null}"
                echo "✓ Kubernetes kubeconfig written to ~/.kube/config"
            fi
        else
            echo "::notice::No KUBECONFIG provided, will use local cluster if available"
        fi
        ;;

    *)
        echo "::error::Unknown provider: $PROVIDER"
        exit 1
        ;;
esac

echo "provider-configured=true" >> "${GITHUB_OUTPUT:-/dev/null}"
