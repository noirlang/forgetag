param(
    [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$RootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$DistDir = Join-Path $RootDir "dist"

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $DistDir "forgetag-windows-x64.msi"
}

# Build with Tauri which handles WiX natively
Push-Location $RootDir
try {
    npm ci
    npx tauri build --config src-tauri/tauri.conf.json --bundles msi
}
finally {
    Pop-Location
}

# Find the built MSI
$tauriMsi = Get-ChildItem (Join-Path $RootDir "src-tauri/target/release/bundle/msi/*.msi") -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $tauriMsi) {
    throw "Tauri MSI build did not produce an MSI file"
}

New-Item $DistDir -ItemType Directory -Force | Out-Null
Copy-Item $tauriMsi.FullName $OutputPath

Write-Host "Windows MSI written to $OutputPath"
