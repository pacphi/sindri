# sindri-helpers.psm1 — PowerShell helper module for Sindri lifecycle
# scripts (ADR-030, v4/docs/script-contract.md).
#
# Imported by every PowerShell phase script via the dispatcher-injected
# env var:
#
#   $ErrorActionPreference = 'Stop'
#   Import-Module $env:SINDRI_HELPERS_PSM1 -Force
#   Sindri-Init
#
# `$env:SINDRI_HELPERS_PSM1` is set by the dispatcher to an absolute
# path. No relative `..` traversal needed.

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

function Sindri-VersionOf {
    param([string]$Name)
    if (-not (Sindri-ToolInstalled $Name)) { return '' }
    $raw = & $Name --version 2>$null
    if (-not $raw) { $raw = & $Name version 2>$null }
    $m = [regex]::Match([string]$raw, '\d+\.\d+(\.\d+)?')
    if ($m.Success) { return $m.Value }
    return ''
}

function Sindri-AtVersion {
    param([string]$Name)
    $want = $env:SINDRI_COMPONENT_VERSION
    $have = Sindri-VersionOf -Name $Name
    if ($want -and $have -and ($have.StartsWith($want) -or $have.Contains($want))) {
        Sindri-Log -Level 'info' -Message "$Name already at $want; skipping"
        Sindri-Emit -Name 'skip' -Detail @{ reason = 'already-installed' }
        Sindri-Emit -Name 'phase-complete' -Detail @{ change = $false }
        return $true
    }
    if ($have) {
        Sindri-Log -Level 'info' -Message ("{0}: {1} -> {2}" -f $Name, $have, $want)
    }
    return $false
}

Export-ModuleMember -Function `
    Sindri-Init, Sindri-Log, Sindri-Emit, Sindri-RequireEnv, `
    Sindri-ToolInstalled, Sindri-VersionOf, Sindri-AtVersion
