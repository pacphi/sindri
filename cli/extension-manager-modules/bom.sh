#!/bin/bash
# bom.sh - Bill of Materials generation and management (declarative)

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Detect environment and source common functions
if [[ -f "/docker/lib/common.sh" ]]; then
    source /docker/lib/common.sh
else
    source "${MODULE_DIR}/../../docker/lib/common.sh"
fi

source "${MODULE_DIR}/dependency.sh"

BOM_DIR="${WORKSPACE_SYSTEM:-/workspace/.system}/bom"

# Generate BOM for a single extension
generate_extension_bom() {
    local ext_name="$1"
    local ext_yaml="$EXTENSIONS_DIR/$ext_name/extension.yaml"

    if [[ ! -f "$ext_yaml" ]]; then
        print_error "Extension YAML not found: $ext_yaml"
        return 1
    fi

    # Check if extension is installed
    if ! is_extension_installed "$ext_name"; then
        print_warning "Extension not installed: $ext_name"
        return 1
    fi

    local bom_file="$BOM_DIR/${ext_name}.bom.yaml"
    ensure_directory "$BOM_DIR"

    # Load metadata
    local version category description
    version=$(load_yaml "$ext_yaml" '.metadata.version')
    category=$(load_yaml "$ext_yaml" '.metadata.category')
    description=$(load_yaml "$ext_yaml" '.metadata.description')

    # Get installation timestamp
    local installed_at
    if [[ -f "${WORKSPACE_SYSTEM:-/workspace/.system}/installed/$ext_name.installed" ]]; then
        installed_at=$(cat "${WORKSPACE_SYSTEM:-/workspace/.system}/installed/$ext_name.installed")
    else
        installed_at="unknown"
    fi

    # Start BOM YAML
    cat > "$bom_file" << EOF
# Bill of Materials for extension: $ext_name
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")

extension:
  name: $ext_name
  version: $version
  category: $category
  description: "$description"
  installed: $installed_at

software: []
files: []
EOF

    # Check if extension has explicit BOM section
    local has_bom
    has_bom=$(load_yaml "$ext_yaml" '.bom' 2>/dev/null || echo "null")

    if [[ "$has_bom" != "null" ]]; then
        # Use explicit BOM from extension.yaml
        extract_explicit_bom "$ext_name" "$ext_yaml" "$bom_file"
    else
        # Derive BOM from install method
        derive_bom_from_install "$ext_name" "$ext_yaml" "$bom_file"
    fi

    # Resolve dynamic versions
    resolve_dynamic_versions "$bom_file"

    [[ "${VERBOSE:-false}" == "true" ]] && print_success "Generated BOM for $ext_name: $bom_file"
}

# Extract explicit BOM from extension.yaml
extract_explicit_bom() {
    local ext_name="$1"
    local ext_yaml="$2"
    local bom_file="$3"

    # Extract tools
    local tools_count
    tools_count=$(load_yaml "$ext_yaml" '.bom.tools | length' 2>/dev/null || echo "0")

    if [[ "$tools_count" != "null" ]] && [[ "$tools_count" -gt 0 ]]; then
        for i in $(seq 0 $((tools_count - 1))); do
            local tool_name tool_version tool_source tool_type tool_license tool_homepage
            tool_name=$(load_yaml "$ext_yaml" ".bom.tools[$i].name")
            tool_version=$(load_yaml "$ext_yaml" ".bom.tools[$i].version" 2>/dev/null || echo "dynamic")
            tool_source=$(load_yaml "$ext_yaml" ".bom.tools[$i].source")
            tool_type=$(load_yaml "$ext_yaml" ".bom.tools[$i].type" 2>/dev/null || echo "utility")
            tool_license=$(load_yaml "$ext_yaml" ".bom.tools[$i].license" 2>/dev/null || echo "")
            tool_homepage=$(load_yaml "$ext_yaml" ".bom.tools[$i].homepage" 2>/dev/null || echo "")

            # Append to BOM file using yq
            if command_exists yq; then
                local entry="{\"name\": \"$tool_name\", \"version\": \"$tool_version\", \"source\": \"$tool_source\", \"type\": \"$tool_type\""
                [[ -n "$tool_license" ]] && [[ "$tool_license" != "null" ]] && entry+=", \"license\": \"$tool_license\""
                [[ -n "$tool_homepage" ]] && [[ "$tool_homepage" != "null" ]] && entry+=", \"homepage\": \"$tool_homepage\""
                entry+="}"

                yq eval -i ".software += [$entry]" "$bom_file"
            fi
        done
    fi

    # Extract files
    local files_count
    files_count=$(load_yaml "$ext_yaml" '.bom.files | length' 2>/dev/null || echo "0")

    if [[ "$files_count" != "null" ]] && [[ "$files_count" -gt 0 ]]; then
        for i in $(seq 0 $((files_count - 1))); do
            local file_path file_type
            file_path=$(load_yaml "$ext_yaml" ".bom.files[$i].path")
            file_type=$(load_yaml "$ext_yaml" ".bom.files[$i].type")

            if command_exists yq; then
                local entry="{\"path\": \"$file_path\", \"type\": \"$file_type\"}"
                yq eval -i ".files += [$entry]" "$bom_file"
            fi
        done
    fi
}

# Derive BOM from install method (auto-discovery)
derive_bom_from_install() {
    local ext_name="$1"
    local ext_yaml="$2"
    local bom_file="$3"

    local install_method
    install_method=$(load_yaml "$ext_yaml" '.install.method')

    case "$install_method" in
        mise)
            derive_bom_from_mise "$ext_name" "$ext_yaml" "$bom_file"
            ;;
        apt)
            derive_bom_from_apt "$ext_name" "$ext_yaml" "$bom_file"
            ;;
        npm)
            derive_bom_from_npm "$ext_name" "$ext_yaml" "$bom_file"
            ;;
        binary)
            derive_bom_from_binary "$ext_name" "$ext_yaml" "$bom_file"
            ;;
        script)
            # Script-based installs need explicit BOM
            [[ "${VERBOSE:-false}" == "true" ]] && print_warning "Script-based extension requires explicit BOM section"
            ;;
        hybrid)
            # Hybrid needs explicit BOM
            [[ "${VERBOSE:-false}" == "true" ]] && print_warning "Hybrid extension requires explicit BOM section"
            ;;
    esac
}

# Derive BOM from mise installation
derive_bom_from_mise() {
    local ext_name="$1"
    local ext_yaml="$2"
    local bom_file="$3"

    local config_file
    config_file=$(load_yaml "$ext_yaml" '.install.mise.configFile')

    if [[ -z "$config_file" ]] || [[ "$config_file" == "null" ]]; then
        return 0
    fi

    local ext_dir
    ext_dir=$(dirname "$ext_yaml")
    local config_path="$ext_dir/$config_file"

    if [[ ! -f "$config_path" ]]; then
        return 0
    fi

    # Parse mise.toml for tools
    local tools
    tools=$(grep -E '^\s*[a-z0-9-]+\s*=' "$config_path" 2>/dev/null | awk '{print $1}' || true)

    for tool in $tools; do
        if command_exists yq; then
            local entry="{\"name\": \"$tool\", \"version\": \"dynamic\", \"source\": \"mise\", \"type\": \"runtime\"}"
            yq eval -i ".software += [$entry]" "$bom_file"
        fi
    done
}

# Derive BOM from apt installation
derive_bom_from_apt() {
    local ext_name="$1"
    local ext_yaml="$2"
    local bom_file="$3"

    local packages
    packages=$(load_yaml "$ext_yaml" '.install.apt.packages[]' 2>/dev/null || true)

    for pkg in $packages; do
        if command_exists yq; then
            local entry="{\"name\": \"$pkg\", \"version\": \"dynamic\", \"source\": \"apt\", \"type\": \"cli-tool\"}"
            yq eval -i ".software += [$entry]" "$bom_file"
        fi
    done
}

# Derive BOM from npm installation
derive_bom_from_npm() {
    local ext_name="$1"
    local ext_yaml="$2"
    local bom_file="$3"

    local packages
    packages=$(load_yaml "$ext_yaml" '.install.npm.packages[]' 2>/dev/null || true)

    for pkg in $packages; do
        if command_exists yq; then
            local entry="{\"name\": \"$pkg\", \"version\": \"dynamic\", \"source\": \"npm\", \"type\": \"cli-tool\"}"
            yq eval -i ".software += [$entry]" "$bom_file"
        fi
    done
}

# Derive BOM from binary installation
derive_bom_from_binary() {
    local ext_name="$1"
    local ext_yaml="$2"
    local bom_file="$3"

    local downloads_count
    downloads_count=$(load_yaml "$ext_yaml" '.install.binary.downloads | length' 2>/dev/null || echo "0")

    if [[ "$downloads_count" != "null" ]] && [[ "$downloads_count" -gt 0 ]]; then
        for i in $(seq 0 $((downloads_count - 1))); do
            local name url
            name=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].name")
            url=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].source.url")

            if command_exists yq; then
                local entry="{\"name\": \"$name\", \"version\": \"dynamic\", \"source\": \"binary\", \"type\": \"cli-tool\", \"downloadUrl\": \"$url\"}"
                yq eval -i ".software += [$entry]" "$bom_file"
            fi
        done
    fi
}

# Resolve dynamic versions to actual installed versions
resolve_dynamic_versions() {
    local bom_file="$1"

    if ! command_exists yq; then
        return 0
    fi

    local software_count
    software_count=$(yq eval '.software | length' "$bom_file" 2>/dev/null || echo "0")

    if [[ "$software_count" == "0" ]] || [[ "$software_count" == "null" ]]; then
        return 0
    fi

    for i in $(seq 0 $((software_count - 1))); do
        local name version
        name=$(yq eval ".software[$i].name" "$bom_file")
        version=$(yq eval ".software[$i].version" "$bom_file")

        if [[ "$version" == "dynamic" ]]; then
            # Try to resolve actual version
            local actual_version=""

            if command_exists "$name"; then
                # Try common version flags
                actual_version=$($name --version 2>&1 | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || true)
                if [[ -z "$actual_version" ]]; then
                    actual_version=$($name -v 2>&1 | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || true)
                fi
                if [[ -z "$actual_version" ]]; then
                    actual_version=$($name version 2>&1 | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 || true)
                fi
            fi

            if [[ -n "$actual_version" ]]; then
                yq eval -i ".software[$i].version = \"$actual_version\"" "$bom_file"
            else
                yq eval -i ".software[$i].version = \"unknown\"" "$bom_file"
            fi
        fi
    done
}

# Generate aggregate BOM for all installed extensions
generate_aggregate_bom() {
    local output_file="${1:-$BOM_DIR/complete.bom.yaml}"
    ensure_directory "$BOM_DIR"

    print_status "Generating aggregate BOM..."

    # Get all installed extensions
    local installed_exts
    installed_exts=$(find "${WORKSPACE_SYSTEM:-/workspace/.system}/installed" -name "*.installed" -exec basename {} .installed \; 2>/dev/null)

    if [[ -z "$installed_exts" ]]; then
        print_warning "No extensions installed"
        return 0
    fi

    # Initialize aggregate BOM
    cat > "$output_file" << EOF
# Aggregate Bill of Materials
# Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
# Sindri Version: 1.0.0

extensions: []
EOF

    # Generate BOM for each extension
    for ext in $installed_exts; do
        generate_extension_bom "$ext"

        # Read extension BOM and add to aggregate
        local ext_bom="$BOM_DIR/${ext}.bom.yaml"
        if [[ -f "$ext_bom" ]] && command_exists yq; then
            yq eval -i ".extensions += [$(yq eval '.' "$ext_bom" -o=json)]" "$output_file"
        fi
    done

    print_success "Aggregate BOM generated: $output_file"
}

# Show BOM for extension or all extensions
show_bom() {
    local ext_name="${1:-}"
    local format="${FORMAT:-yaml}"

    if [[ -z "$ext_name" ]]; then
        # Show aggregate BOM
        local aggregate_bom="$BOM_DIR/complete.bom.yaml"
        if [[ ! -f "$aggregate_bom" ]]; then
            generate_aggregate_bom "$aggregate_bom"
        fi

        export_bom_format "$aggregate_bom" "$format"
    else
        # Show specific extension BOM
        local ext_bom="$BOM_DIR/${ext_name}.bom.yaml"
        if [[ ! -f "$ext_bom" ]]; then
            generate_extension_bom "$ext_name"
        fi

        if [[ -f "$ext_bom" ]]; then
            export_bom_format "$ext_bom" "$format"
        else
            print_error "BOM not found for: $ext_name"
            return 1
        fi
    fi
}

# Export BOM in different formats
export_bom_format() {
    local bom_file="$1"
    local format="$2"

    case "$format" in
        yaml|yml)
            cat "$bom_file"
            ;;
        json)
            if command_exists yq; then
                yq eval -o=json "$bom_file"
            else
                print_error "yq required for JSON export"
                return 1
            fi
            ;;
        csv)
            export_bom_csv "$bom_file"
            ;;
        cyclonedx)
            export_bom_cyclonedx "$bom_file"
            ;;
        spdx)
            export_bom_spdx "$bom_file"
            ;;
        *)
            print_error "Unknown format: $format"
            return 1
            ;;
    esac
}

# Export BOM as CSV
export_bom_csv() {
    local bom_file="$1"

    if ! command_exists yq; then
        print_error "yq required for CSV export"
        return 1
    fi

    echo "Extension,Software,Version,Source,Type,License"

    # Check if this is aggregate or single extension BOM
    local is_aggregate
    is_aggregate=$(yq eval 'has("extensions")' "$bom_file")

    if [[ "$is_aggregate" == "true" ]]; then
        # Aggregate BOM
        local ext_count
        ext_count=$(yq eval '.extensions | length' "$bom_file")

        for i in $(seq 0 $((ext_count - 1))); do
            local ext_name
            ext_name=$(yq eval ".extensions[$i].extension.name" "$bom_file")

            local sw_count
            sw_count=$(yq eval ".extensions[$i].software | length" "$bom_file")

            for j in $(seq 0 $((sw_count - 1))); do
                local sw_name sw_version sw_source sw_type sw_license
                sw_name=$(yq eval ".extensions[$i].software[$j].name" "$bom_file")
                sw_version=$(yq eval ".extensions[$i].software[$j].version" "$bom_file")
                sw_source=$(yq eval ".extensions[$i].software[$j].source" "$bom_file")
                sw_type=$(yq eval ".extensions[$i].software[$j].type" "$bom_file")
                sw_license=$(yq eval ".extensions[$i].software[$j].license" "$bom_file" 2>/dev/null || echo "")

                echo "$ext_name,$sw_name,$sw_version,$sw_source,$sw_type,$sw_license"
            done
        done
    else
        # Single extension BOM
        local ext_name
        ext_name=$(yq eval ".extension.name" "$bom_file")

        local sw_count
        sw_count=$(yq eval '.software | length' "$bom_file")

        for i in $(seq 0 $((sw_count - 1))); do
            local sw_name sw_version sw_source sw_type sw_license
            sw_name=$(yq eval ".software[$i].name" "$bom_file")
            sw_version=$(yq eval ".software[$i].version" "$bom_file")
            sw_source=$(yq eval ".software[$i].source" "$bom_file")
            sw_type=$(yq eval ".software[$i].type" "$bom_file")
            sw_license=$(yq eval ".software[$i].license" "$bom_file" 2>/dev/null || echo "")

            echo "$ext_name,$sw_name,$sw_version,$sw_source,$sw_type,$sw_license"
        done
    fi
}

# Export BOM as CycloneDX SBOM
export_bom_cyclonedx() {
    local bom_file="$1"

    if ! command_exists yq; then
        print_error "yq required for CycloneDX export"
        return 1
    fi

    cat << 'EOF'
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.4",
  "version": 1,
  "metadata": {
    "timestamp": "
EOF
    date -u +"%Y-%m-%dT%H:%M:%SZ" | tr -d '\n'
    cat << 'EOF'
",
    "component": {
      "type": "application",
      "name": "sindri-workspace",
      "version": "1.0.0"
    }
  },
  "components": [
EOF

    # Extract components
    local is_aggregate
    is_aggregate=$(yq eval 'has("extensions")' "$bom_file")

    local first=true

    if [[ "$is_aggregate" == "true" ]]; then
        local ext_count
        ext_count=$(yq eval '.extensions | length' "$bom_file")

        for i in $(seq 0 $((ext_count - 1))); do
            local sw_count
            sw_count=$(yq eval ".extensions[$i].software | length" "$bom_file")

            for j in $(seq 0 $((sw_count - 1))); do
                local sw_name sw_version sw_type sw_license
                sw_name=$(yq eval ".extensions[$i].software[$j].name" "$bom_file")
                sw_version=$(yq eval ".extensions[$i].software[$j].version" "$bom_file")
                sw_type=$(yq eval ".extensions[$i].software[$j].type" "$bom_file")
                sw_license=$(yq eval ".extensions[$i].software[$j].license" "$bom_file" 2>/dev/null || echo "")

                [[ "$first" == "false" ]] && echo ","
                first=false

                cat << EOF
    {
      "type": "library",
      "name": "$sw_name",
      "version": "$sw_version"
EOF
                if [[ -n "$sw_license" ]] && [[ "$sw_license" != "null" ]]; then
                    cat << EOF
,
      "licenses": [
        {
          "license": {
            "id": "$sw_license"
          }
        }
      ]
EOF
                fi
                echo -n "    }"
            done
        done
    else
        local sw_count
        sw_count=$(yq eval '.software | length' "$bom_file")

        for i in $(seq 0 $((sw_count - 1))); do
            local sw_name sw_version sw_type sw_license
            sw_name=$(yq eval ".software[$i].name" "$bom_file")
            sw_version=$(yq eval ".software[$i].version" "$bom_file")
            sw_type=$(yq eval ".software[$i].type" "$bom_file")
            sw_license=$(yq eval ".software[$i].license" "$bom_file" 2>/dev/null || echo "")

            [[ "$first" == "false" ]] && echo ","
            first=false

            cat << EOF
    {
      "type": "library",
      "name": "$sw_name",
      "version": "$sw_version"
EOF
            if [[ -n "$sw_license" ]] && [[ "$sw_license" != "null" ]]; then
                cat << EOF
,
      "licenses": [
        {
          "license": {
            "id": "$sw_license"
          }
        }
      ]
EOF
            fi
            echo -n "    }"
        done
    fi

    cat << 'EOF'

  ]
}
EOF
}

# Export BOM as SPDX SBOM
export_bom_spdx() {
    local bom_file="$1"

    if ! command_exists yq; then
        print_error "yq required for SPDX export"
        return 1
    fi

    cat << EOF
SPDXVersion: SPDX-2.3
DataLicense: CC0-1.0
SPDXID: SPDXRef-DOCUMENT
DocumentName: Sindri-Workspace-BOM
DocumentNamespace: https://sindri.dev/spdxdocs/workspace-$(date +%s)
Creator: Tool: sindri-extension-manager

EOF

    echo "# Packages"
    echo ""

    local is_aggregate
    is_aggregate=$(yq eval 'has("extensions")' "$bom_file")

    local pkg_num=1

    if [[ "$is_aggregate" == "true" ]]; then
        local ext_count
        ext_count=$(yq eval '.extensions | length' "$bom_file")

        for i in $(seq 0 $((ext_count - 1))); do
            local sw_count
            sw_count=$(yq eval ".extensions[$i].software | length" "$bom_file")

            for j in $(seq 0 $((sw_count - 1))); do
                local sw_name sw_version sw_license
                sw_name=$(yq eval ".extensions[$i].software[$j].name" "$bom_file")
                sw_version=$(yq eval ".extensions[$i].software[$j].version" "$bom_file")
                sw_license=$(yq eval ".extensions[$i].software[$j].license" "$bom_file" 2>/dev/null || echo "NOASSERTION")

                cat << EOF
PackageName: $sw_name
SPDXID: SPDXRef-Package-$pkg_num
PackageVersion: $sw_version
PackageDownloadLocation: NOASSERTION
FilesAnalyzed: false
PackageLicenseConcluded: $sw_license
PackageLicenseDeclared: $sw_license
PackageCopyrightText: NOASSERTION

EOF
                ((pkg_num++))
            done
        done
    else
        local sw_count
        sw_count=$(yq eval '.software | length' "$bom_file")

        for i in $(seq 0 $((sw_count - 1))); do
            local sw_name sw_version sw_license
            sw_name=$(yq eval ".software[$i].name" "$bom_file")
            sw_version=$(yq eval ".software[$i].version" "$bom_file")
            sw_license=$(yq eval ".software[$i].license" "$bom_file" 2>/dev/null || echo "NOASSERTION")

            cat << EOF
PackageName: $sw_name
SPDXID: SPDXRef-Package-$pkg_num
PackageVersion: $sw_version
PackageDownloadLocation: NOASSERTION
FilesAnalyzed: false
PackageLicenseConcluded: $sw_license
PackageLicenseDeclared: $sw_license
PackageCopyrightText: NOASSERTION

EOF
            ((pkg_num++))
        done
    fi
}

# Regenerate all BOMs
regenerate_all_boms() {
    print_status "Regenerating all BOMs..."

    local installed_exts
    installed_exts=$(find "${WORKSPACE_SYSTEM:-/workspace/.system}/installed" -name "*.installed" -exec basename {} .installed \; 2>/dev/null)

    if [[ -z "$installed_exts" ]]; then
        print_warning "No extensions installed"
        return 0
    fi

    for ext in $installed_exts; do
        generate_extension_bom "$ext"
    done

    generate_aggregate_bom

    print_success "All BOMs regenerated"
}

# Export functions
export -f generate_extension_bom generate_aggregate_bom show_bom
export -f export_bom_format export_bom_csv export_bom_cyclonedx export_bom_spdx
export -f regenerate_all_boms
export -f extract_explicit_bom derive_bom_from_install resolve_dynamic_versions
export -f derive_bom_from_mise derive_bom_from_apt derive_bom_from_npm derive_bom_from_binary
