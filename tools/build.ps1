param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
$nullgravRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $nullgravRoot
try {
    if (-not $SkipTests) {
        cargo test --locked
        if ($LASTEXITCODE -ne 0) { throw 'Host tests failed.' }
    }
    cargo build --locked --release -p nullgrav-boot --target x86_64-unknown-uefi
    if ($LASTEXITCODE -ne 0) { throw 'UEFI build failed.' }
    cargo build --locked --release -p fieldctl
    if ($LASTEXITCODE -ne 0) { throw 'fieldctl build failed.' }
    python (Join-Path $PSScriptRoot 'build_image.py')
    if ($LASTEXITCODE -ne 0) { throw 'Image packaging failed.' }
    Copy-Item -LiteralPath (Join-Path $nullgravRoot 'target\release\fieldctl.exe') -Destination (Join-Path $nullgravRoot 'dist\fieldctl.exe')
    Copy-Item -LiteralPath (Join-Path $nullgravRoot 'target\x86_64-unknown-uefi\release\nullgrav-boot.efi') -Destination (Join-Path $nullgravRoot 'dist\BOOTX64.EFI')
} finally { Pop-Location }
