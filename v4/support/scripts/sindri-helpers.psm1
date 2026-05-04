# sindri-helpers.psm1 — PowerShell helper module for Sindri lifecycle
# scripts (ADR-030, v4/docs/script-contract.md).
#
# Usage in a phase script:
#
#   $ErrorActionPreference = 'Stop'
#   Import-Module (Join-Path $PSScriptRoot '../../../support/scripts/sindri-helpers.psm1') -Force
#   Sindri-Init
#
#   if (Sindri-ToolInstalled 'mytool') {
#       Sindri-Emit phase-complete @{ change = $false }
#       exit 0
#   }
#
#   # …do the install…
#   Sindri-Log info "installed mytool $env:SINDRI_COMPONENT_VERSION"
#   Sindri-Emit phase-complete @{ change = $true }

function Sindri-RequireEnv {
    param([string[]]$Names)
    $missing = @()
    foreach ($n in $Names) {
        if (-not [string]::IsNullOrEmpty([Environment]::GetEnvironmentVariable($n))) {
            continue
        }
        $missing += $n
    }
    if ($missing.Count -gt 0) {
        Write-Error "sindri-helpers: missing required env vars: $($missing -join ', ')"
        exit 64
    }
}

function Sindri-Init {
    Sindri-RequireEnv -Names @(
        'SINDRI_PHASE',
        'SINDRI_COMPONENT_ADDRESS',
        'SINDRI_COMPONENT_VERSION',
        'SINDRI_TARGET',
        'SINDRI_LOG_DIR',
        'SINDRI_EVENTS'
    )

    if (-not (Test-Path -LiteralPath $env:SINDRI_LOG_DIR)) {
        New-Item -ItemType Directory -Path $env:SINDRI_LOG_DIR -Force | Out-Null
    }
    # Truncate events file.
    Set-Content -LiteralPath $env:SINDRI_EVENTS -Value '' -NoNewline -ErrorAction SilentlyContinue

    $prior = if ([string]::IsNullOrEmpty($env:SINDRI_PRIOR_VERSION)) { '<none>' } else { $env:SINDRI_PRIOR_VERSION }
    Sindri-Log -Level 'info' -Message "phase=$($env:SINDRI_PHASE) component=$($env:SINDRI_COMPONENT_ADDRESS) version=$($env:SINDRI_COMPONENT_VERSION) prior=$prior dry_run=$($env:SINDRI_DRY_RUN)"
}

function Sindri-Log {
    param(
        [Parameter(Mandatory = $true)][string]$Level,
        [Parameter(Mandatory = $true)][string]$Message
    )
    $phase = if ([string]::IsNullOrEmpty($env:SINDRI_PHASE)) { '?' } else { $env:SINDRI_PHASE }
    [Console]::Error.WriteLine("[sindri $phase] $Level`: $Message")
}

function Sindri-Emit {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $false)][hashtable]$Detail = @{}
    )
    if ([string]::IsNullOrEmpty($env:SINDRI_EVENTS)) {
        return
    }
    $merged = @{ event = $Name }
    foreach ($k in $Detail.Keys) {
        $merged[$k] = $Detail[$k]
    }
    $json = $merged | ConvertTo-Json -Compress -Depth 10
    Add-Content -LiteralPath $env:SINDRI_EVENTS -Value $json
}

function Sindri-ToolInstalled {
    param([string]$Name)
    return [bool](Get-Command -Name $Name -ErrorAction SilentlyContinue)
}

Export-ModuleMember -Function Sindri-Init, Sindri-Log, Sindri-Emit, Sindri-RequireEnv, Sindri-ToolInstalled
