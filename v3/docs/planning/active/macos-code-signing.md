# macOS Code Signing & Notarization Plan

## Status: Dormant (Awaiting Apple Developer Account)

This plan describes how to implement macOS code signing and notarization for Sindri v3 CLI binaries using `rcodesign` (a pure Rust implementation). The workflow is designed to be dormant until Apple Developer secrets are configured.

## Problem Statement

Without code signing and notarization, macOS users see warnings when downloading Sindri binaries:

- "sindri cannot be opened because the developer cannot be verified"
- "sindri is damaged and can't be opened"

## Solution Overview

Use `rcodesign` to sign macOS binaries with a Developer ID Application certificate and submit them to Apple's notarization service. This runs on Linux runners (no macOS runner required for signing).

## Prerequisites

- Apple Developer Program membership ($99/year)
- Developer ID Application certificate
- App Store Connect API key for notarization

## Why rcodesign?

- **Cross-platform**: Runs on Linux, no macOS runner required
- **Open source**: No proprietary Apple tools needed
- **Supports notarization**: Can submit to Apple's notary service
- **Well-maintained**: Active development by Gregory Szorc

## Required GitHub Secrets

| Secret                      | Description                          | Format                                                                 |
| --------------------------- | ------------------------------------ | ---------------------------------------------------------------------- |
| `APPLE_CERTIFICATE_PEM`     | Developer ID Application certificate | PEM, base64-encoded                                                    |
| `APPLE_CERTIFICATE_KEY_PEM` | Certificate private key              | PEM, base64-encoded                                                    |
| `APPLE_NOTARY_KEY_JSON`     | App Store Connect API key            | JSON from `rcodesign encode-app-store-connect-api-key`, base64-encoded |

## Implementation

### File: `.github/workflows/macos-codesign.yml`

Create a reusable workflow that other workflows can call:

```yaml
name: "macOS Code Signing (Reusable)"

on:
  workflow_call:
    inputs:
      version:
        required: true
        type: string
        description: "Version being released (e.g., 3.0.0)"
    secrets:
      APPLE_CERTIFICATE_PEM:
        required: false
        description: "Developer ID Application certificate (PEM format, base64)"
      APPLE_CERTIFICATE_KEY_PEM:
        required: false
        description: "Certificate private key (PEM format, base64)"
      APPLE_NOTARY_KEY_JSON:
        required: false
        description: "App Store Connect API key (JSON format)"
    outputs:
      signed:
        description: "Whether binaries were signed"
        value: ${{ jobs.check-secrets.outputs.can_sign }}

jobs:
  check-secrets:
    runs-on: ubuntu-latest
    outputs:
      can_sign: ${{ steps.check.outputs.can_sign }}
    steps:
      - name: Check if signing secrets are configured
        id: check
        run: |
          if [[ -n "${{ secrets.APPLE_CERTIFICATE_PEM }}" ]] && \
             [[ -n "${{ secrets.APPLE_CERTIFICATE_KEY_PEM }}" ]] && \
             [[ -n "${{ secrets.APPLE_NOTARY_KEY_JSON }}" ]]; then
            echo "can_sign=true" >> $GITHUB_OUTPUT
            echo "✅ Apple signing secrets configured - will sign binaries"
          else
            echo "can_sign=false" >> $GITHUB_OUTPUT
            echo "⏭️ Apple signing secrets not configured - skipping code signing"
            echo "To enable signing, configure these repository secrets:"
            echo "  - APPLE_CERTIFICATE_PEM"
            echo "  - APPLE_CERTIFICATE_KEY_PEM"
            echo "  - APPLE_NOTARY_KEY_JSON"
          fi

  sign-macos:
    name: Sign ${{ matrix.platform }}
    runs-on: ubuntu-latest
    needs: check-secrets
    if: needs.check-secrets.outputs.can_sign == 'true'
    strategy:
      matrix:
        include:
          - platform: macos-x86_64
          - platform: macos-aarch64
    steps:
      - uses: actions/checkout@v6

      - name: Install rcodesign
        run: |
          cargo install apple-codesign
          rcodesign --version

      - name: Download unsigned binary
        uses: actions/download-artifact@v7
        with:
          name: binary-${{ matrix.platform }}
          path: unsigned

      - name: Extract binary
        run: |
          cd unsigned
          tar -xzf sindri-*.tar.gz

      - name: Setup signing credentials
        run: |
          echo "${{ secrets.APPLE_CERTIFICATE_PEM }}" | base64 -d > cert.pem
          echo "${{ secrets.APPLE_CERTIFICATE_KEY_PEM }}" | base64 -d > key.pem
          echo "${{ secrets.APPLE_NOTARY_KEY_JSON }}" | base64 -d > notary-key.json

      - name: Sign binary with hardened runtime
        run: |
          rcodesign sign \
            --pem-source cert.pem \
            --pem-source key.pem \
            --code-signature-flags runtime \
            unsigned/sindri

          echo "✅ Binary signed"
          rcodesign analyze unsigned/sindri

      - name: Create notarization zip
        run: |
          cd unsigned
          zip sindri.zip sindri

      - name: Submit for notarization
        run: |
          rcodesign notary-submit \
            --api-key-path notary-key.json \
            --wait \
            unsigned/sindri.zip

          echo "✅ Notarization successful"

      - name: Repackage signed binary
        run: |
          VERSION="${{ inputs.version }}"
          cd unsigned
          tar -czf sindri-v${VERSION}-${{ matrix.platform }}.tar.gz sindri
          mv sindri-v${VERSION}-${{ matrix.platform }}.tar.gz ../

      - name: Upload signed binary
        uses: actions/upload-artifact@v6
        with:
          name: binary-${{ matrix.platform }}-signed
          path: sindri-*.tar.gz
          retention-days: 1

      - name: Cleanup credentials
        if: always()
        run: rm -f cert.pem key.pem notary-key.json
```

### Changes to `.github/workflows/release-v3.yml`

Add call to reusable workflow after build-binaries:

```yaml
sign-macos-binaries:
  needs: [validate-tag, build-binaries]
  uses: ./.github/workflows/macos-codesign.yml
  with:
    version: ${{ needs.validate-tag.outputs.version }}
  secrets:
    APPLE_CERTIFICATE_PEM: ${{ secrets.APPLE_CERTIFICATE_PEM }}
    APPLE_CERTIFICATE_KEY_PEM: ${{ secrets.APPLE_CERTIFICATE_KEY_PEM }}
    APPLE_NOTARY_KEY_JSON: ${{ secrets.APPLE_NOTARY_KEY_JSON }}
```

Update create-release job needs and asset preparation:

```yaml
create-release:
  needs: [validate-tag, generate-changelog, build-binaries, sign-macos-binaries, ...]

  # In "Prepare release assets" step, add:
  - name: Prepare release assets (prefer signed macOS binaries)
    run: |
      mkdir -p release-assets
      find artifacts -name "sindri-*.tar.gz" -exec cp {} release-assets/ \;
      find artifacts -name "sindri-*.zip" -exec cp {} release-assets/ \;

      # Overwrite with signed macOS binaries if available
      for platform in macos-x86_64 macos-aarch64; do
        signed_dir="artifacts/binary-${platform}-signed"
        if [[ -d "$signed_dir" ]]; then
          echo "✅ Using signed binary for $platform"
          find "$signed_dir" -name "sindri-*.tar.gz" -exec cp {} release-assets/ \;
        else
          echo "⚠️ Using unsigned binary for $platform"
        fi
      done
```

## Apple Developer Account Setup

### Step 1: Enroll in Apple Developer Program

1. Go to https://developer.apple.com/programs/enroll/
2. Enroll as Individual ($99/year) or Organization
3. Wait for approval (usually 24-48 hours)

### Step 2: Create Developer ID Application Certificate

1. Go to https://developer.apple.com/account/resources/certificates/list
2. Click "+" to create new certificate
3. Select "Developer ID Application"
4. Create Certificate Signing Request (CSR) using Keychain Access or openssl
5. Upload CSR and download certificate
6. Convert to PEM format for rcodesign

### Step 3: Create App Store Connect API Key

1. Go to https://appstoreconnect.apple.com/access/api
2. Click "+" under "Keys"
3. Name: "Sindri Notarization", Access: "Developer"
4. Download .p8 key file (one-time download!)
5. Note Key ID and Issuer ID
6. Use `rcodesign encode-app-store-connect-api-key` to create JSON

### Step 4: Configure GitHub Secrets

```bash
# Base64 encode and add to GitHub secrets
cat cert.pem | base64 | pbcopy          # APPLE_CERTIFICATE_PEM
cat key.pem | base64 | pbcopy           # APPLE_CERTIFICATE_KEY_PEM
cat notary-key.json | base64 | pbcopy   # APPLE_NOTARY_KEY_JSON
```

## Verification

Once enabled, users can verify signed binaries:

```bash
# Check signature (on macOS)
codesign -dv --verbose=4 $(which sindri)

# Check notarization
spctl --assess --type execute $(which sindri)
```

## Sources

- [rcodesign Documentation](https://gregoryszorc.com/docs/apple-codesign/stable/)
- [Apple Code Sign Action (indygreg)](https://github.com/indygreg/apple-code-sign-action)
- [A Very Rough Guide to Notarizing CLI Apps](https://www.randomerrata.com/articles/2024/notarize/)
- [Federico Terzi - Automatic Code Signing for macOS](https://federicoterzi.com/blog/automatic-code-signing-and-notarization-for-macos-apps-using-github-actions/)
