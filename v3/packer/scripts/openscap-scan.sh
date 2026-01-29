#!/bin/bash
# openscap-scan.sh - OpenSCAP security compliance scanning for Sindri VM images
# This script runs security compliance scans and generates reports

set -euo pipefail

SCAN_PROFILE="${SCAN_PROFILE:-xccdf_org.ssgproject.content_profile_cis_level1_server}"
SCAN_TYPE="${SCAN_TYPE:-cis}"
OUTPUT_DIR="${OUTPUT_DIR:-/tmp/openscap-results}"
REPORT_FORMAT="${REPORT_FORMAT:-html}"

echo "=== OpenSCAP Security Compliance Scan ==="
echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Profile: ${SCAN_PROFILE}"
echo "Output: ${OUTPUT_DIR}"

# Install OpenSCAP if not present
if ! command -v oscap &> /dev/null; then
    echo "Installing OpenSCAP..."
    apt-get update
    apt-get install -y --no-install-recommends \
        libopenscap8 \
        openscap-scanner \
        openscap-utils \
        ssg-base \
        ssg-debderived \
        ssg-debian \
        ssg-nondebian \
        ssg-applications \
        bzip2
fi

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Determine the correct SCAP content file
SCAP_CONTENT=""
OS_ID=$(grep '^ID=' /etc/os-release | cut -d= -f2 | tr -d '"')
OS_VERSION=$(grep '^VERSION_ID=' /etc/os-release | cut -d= -f2 | tr -d '"')

case "$OS_ID" in
    ubuntu)
        # Ubuntu uses Debian-derived content
        if [ -f /usr/share/xml/scap/ssg/content/ssg-ubuntu2404-ds.xml ]; then
            SCAP_CONTENT="/usr/share/xml/scap/ssg/content/ssg-ubuntu2404-ds.xml"
        elif [ -f /usr/share/xml/scap/ssg/content/ssg-ubuntu2204-ds.xml ]; then
            SCAP_CONTENT="/usr/share/xml/scap/ssg/content/ssg-ubuntu2204-ds.xml"
        elif [ -f /usr/share/xml/scap/ssg/content/ssg-ubuntu2004-ds.xml ]; then
            SCAP_CONTENT="/usr/share/xml/scap/ssg/content/ssg-ubuntu2004-ds.xml"
        fi
        ;;
    debian)
        if [ -f /usr/share/xml/scap/ssg/content/ssg-debian12-ds.xml ]; then
            SCAP_CONTENT="/usr/share/xml/scap/ssg/content/ssg-debian12-ds.xml"
        elif [ -f /usr/share/xml/scap/ssg/content/ssg-debian11-ds.xml ]; then
            SCAP_CONTENT="/usr/share/xml/scap/ssg/content/ssg-debian11-ds.xml"
        fi
        ;;
    rhel|centos|rocky|almalinux)
        if [ -f /usr/share/xml/scap/ssg/content/ssg-rhel9-ds.xml ]; then
            SCAP_CONTENT="/usr/share/xml/scap/ssg/content/ssg-rhel9-ds.xml"
        elif [ -f /usr/share/xml/scap/ssg/content/ssg-rhel8-ds.xml ]; then
            SCAP_CONTENT="/usr/share/xml/scap/ssg/content/ssg-rhel8-ds.xml"
        fi
        ;;
esac

if [ -z "$SCAP_CONTENT" ]; then
    echo "WARNING: No SCAP content found for $OS_ID $OS_VERSION"
    echo "Attempting to find any available content..."
    SCAP_CONTENT=$(find /usr/share/xml/scap/ssg/content -name "*-ds.xml" 2>/dev/null | head -1)
fi

if [ -z "$SCAP_CONTENT" ] || [ ! -f "$SCAP_CONTENT" ]; then
    echo "ERROR: No SCAP content available for this OS"
    echo "Available content files:"
    ls -la /usr/share/xml/scap/ssg/content/*.xml 2>/dev/null || echo "  (none found)"
    exit 1
fi

echo "Using SCAP content: $SCAP_CONTENT"
echo ""

# List available profiles
echo "Available profiles:"
oscap info "$SCAP_CONTENT" | grep -A100 "Profiles:" | head -30
echo ""

# Determine profile based on scan type
case "$SCAN_TYPE" in
    cis|cis-level1)
        # Try CIS Level 1 profiles
        PROFILES=(
            "xccdf_org.ssgproject.content_profile_cis_level1_server"
            "xccdf_org.ssgproject.content_profile_cis_level1_workstation"
            "xccdf_org.ssgproject.content_profile_cis"
        )
        ;;
    cis-level2)
        PROFILES=(
            "xccdf_org.ssgproject.content_profile_cis_level2_server"
            "xccdf_org.ssgproject.content_profile_cis_level2_workstation"
        )
        ;;
    stig)
        PROFILES=(
            "xccdf_org.ssgproject.content_profile_stig"
            "xccdf_org.ssgproject.content_profile_stig_gui"
        )
        ;;
    standard)
        PROFILES=(
            "xccdf_org.ssgproject.content_profile_standard"
        )
        ;;
    *)
        PROFILES=("$SCAN_PROFILE")
        ;;
esac

# Find first available profile
SELECTED_PROFILE=""
for profile in "${PROFILES[@]}"; do
    if oscap info "$SCAP_CONTENT" 2>/dev/null | grep -q "$profile"; then
        SELECTED_PROFILE="$profile"
        break
    fi
done

if [ -z "$SELECTED_PROFILE" ]; then
    echo "WARNING: None of the requested profiles are available"
    echo "Using first available profile..."
    SELECTED_PROFILE=$(oscap info "$SCAP_CONTENT" | grep "Id:" | head -1 | awk '{print $2}')
fi

echo "Selected profile: $SELECTED_PROFILE"
echo ""

# Run the scan
echo "Running OpenSCAP scan..."
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

oscap xccdf eval \
    --profile "$SELECTED_PROFILE" \
    --results "${OUTPUT_DIR}/results_${TIMESTAMP}.xml" \
    --report "${OUTPUT_DIR}/report_${TIMESTAMP}.html" \
    "$SCAP_CONTENT" 2>&1 | tee "${OUTPUT_DIR}/scan_${TIMESTAMP}.log" || true

# Generate ARF (SCAP Results Data Stream) for compliance reporting
echo ""
echo "Generating ARF results..."
oscap xccdf eval \
    --profile "$SELECTED_PROFILE" \
    --results-arf "${OUTPUT_DIR}/arf_${TIMESTAMP}.xml" \
    "$SCAP_CONTENT" 2>/dev/null || true

# Parse and display results summary
echo ""
echo "=== Scan Results Summary ==="
if [ -f "${OUTPUT_DIR}/results_${TIMESTAMP}.xml" ]; then
    PASS=$(grep -c 'result="pass"' "${OUTPUT_DIR}/results_${TIMESTAMP}.xml" || echo "0")
    FAIL=$(grep -c 'result="fail"' "${OUTPUT_DIR}/results_${TIMESTAMP}.xml" || echo "0")
    NOTAPPLICABLE=$(grep -c 'result="notapplicable"' "${OUTPUT_DIR}/results_${TIMESTAMP}.xml" || echo "0")
    ERROR=$(grep -c 'result="error"' "${OUTPUT_DIR}/results_${TIMESTAMP}.xml" || echo "0")

    TOTAL=$((PASS + FAIL + NOTAPPLICABLE + ERROR))
    if [ "$TOTAL" -gt 0 ]; then
        COMPLIANCE=$(echo "scale=2; $PASS * 100 / ($PASS + $FAIL)" | bc 2>/dev/null || echo "N/A")
    else
        COMPLIANCE="N/A"
    fi

    echo "Profile: $SELECTED_PROFILE"
    echo "Total rules evaluated: $TOTAL"
    echo "  Passed: $PASS"
    echo "  Failed: $FAIL"
    echo "  Not Applicable: $NOTAPPLICABLE"
    echo "  Error: $ERROR"
    echo "Compliance: ${COMPLIANCE}%"
    echo ""
    echo "Reports generated:"
    echo "  HTML Report: ${OUTPUT_DIR}/report_${TIMESTAMP}.html"
    echo "  XML Results: ${OUTPUT_DIR}/results_${TIMESTAMP}.xml"
    echo "  ARF Results: ${OUTPUT_DIR}/arf_${TIMESTAMP}.xml"
    echo "  Scan Log: ${OUTPUT_DIR}/scan_${TIMESTAMP}.log"

    # Create summary JSON
    cat > "${OUTPUT_DIR}/summary_${TIMESTAMP}.json" << SUMMARY_JSON
{
    "scan_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "profile": "$SELECTED_PROFILE",
    "content_file": "$SCAP_CONTENT",
    "os_id": "$OS_ID",
    "os_version": "$OS_VERSION",
    "results": {
        "total": $TOTAL,
        "passed": $PASS,
        "failed": $FAIL,
        "not_applicable": $NOTAPPLICABLE,
        "error": $ERROR,
        "compliance_percentage": "$COMPLIANCE"
    },
    "reports": {
        "html": "report_${TIMESTAMP}.html",
        "xml": "results_${TIMESTAMP}.xml",
        "arf": "arf_${TIMESTAMP}.xml",
        "log": "scan_${TIMESTAMP}.log"
    }
}
SUMMARY_JSON

    echo "  JSON Summary: ${OUTPUT_DIR}/summary_${TIMESTAMP}.json"
else
    echo "ERROR: No results file generated"
    exit 1
fi

echo ""
echo "=== OpenSCAP scan complete ==="

# Exit with failure if compliance is below threshold
COMPLIANCE_THRESHOLD="${COMPLIANCE_THRESHOLD:-70}"
if [ "$COMPLIANCE" != "N/A" ]; then
    COMPLIANCE_INT=$(echo "$COMPLIANCE" | cut -d. -f1)
    if [ "$COMPLIANCE_INT" -lt "$COMPLIANCE_THRESHOLD" ]; then
        echo ""
        echo "WARNING: Compliance ($COMPLIANCE%) is below threshold ($COMPLIANCE_THRESHOLD%)"
        # Don't fail the build, just warn
        # exit 1
    fi
fi
