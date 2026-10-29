param([switch]$SkipTests)
$ErrorActionPreference = 'Stop'
$xevuRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $xevuRoot
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
    Copy-Item -LiteralPath (Join-Path $xevuRoot 'target\release\xevuctl.exe') -Destination (Join-Path $xevuRoot 'dist\xevuctl.exe')
    Copy-Item -LiteralPath (Join-Path $xevuRoot 'target\x86_64-unknown-uefi\release\nullgrav-boot.efi') -Destination (Join-Path $xevuRoot 'dist\xevu-BOOTX64.EFI')
} finally { Pop-Location }
