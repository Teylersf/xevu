param([string[]]$Files, [string]$SessionDirectory)
$ErrorActionPreference = 'Stop'
$xevuImportRoot = Split-Path -Parent $PSScriptRoot
Write-Host 'XEVU / ADD FILES TO THE RUNNING TEST VM' -ForegroundColor Green
Write-Host 'Select files. A read-only virtual transfer drive will appear in Disk Gate.'
Write-Host 'The transfer image is a plaintext copy on this Windows computer. Originals stay unchanged.' -ForegroundColor Yellow
Write-Host 'This preview has about 250 MiB total vault space and a 128 MiB per-file limit.'
if (-not $Files -or $Files.Count -eq 0) {
    Add-Type -AssemblyName System.Windows.Forms
    $xevuFileDialog = New-Object System.Windows.Forms.OpenFileDialog
    $xevuFileDialog.Title = 'Choose files to copy into the XEVU test VM'
    $xevuFileDialog.Filter = 'All files (*.*)|*.*'
    $xevuFileDialog.Multiselect = $true
    $xevuFileDialog.CheckFileExists = $true
    try {
        if ($xevuFileDialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) {
            Write-Host 'Selection cancelled. No transfer drive was created.'
            exit 0
        }
        $Files = @($xevuFileDialog.FileNames)
    } finally { $xevuFileDialog.Dispose() }
}
$xevuImportPrivate = Join-Path $xevuImportRoot 'test-output\windows-imports'
$null = New-Item -ItemType Directory -Path $xevuImportPrivate -Force
$xevuImportManifest = Join-Path $xevuImportPrivate ('selection-' + [guid]::NewGuid().ToString('N') + '.json')
try {
    @{ files = @($Files) } | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $xevuImportManifest -Encoding UTF8
    $xevuImportArguments = @((Join-Path $PSScriptRoot 'vm_import.py'), 'load', '--manifest', $xevuImportManifest)
    if ($SessionDirectory) { $xevuImportArguments += @('--session-dir', $SessionDirectory) }
    & python @nullgravImportArguments
    if ($LASTEXITCODE -ne 0) { throw 'Files were not attached. See the error above.' }
} finally {
    # Remove only our temporary list of selected paths; transfer images are kept.
    Remove-Item -LiteralPath $xevuImportManifest -Force -ErrorAction SilentlyContinue
}
Write-Host ''
Write-Host 'IN THE XEVU WINDOW:' -ForegroundColor Green
Write-Host '1. N creates a vault; U unlocks an existing one. Do not guess its passphrase.'
Write-Host '2. Press 3, then R. Select the LAST non-boot FAT volume with the arrow keys.'
Write-Host '3. Press Enter. Select your file and press Enter to encrypt and import it.'
Write-Host '4. Press 2 to see the vault. Repeat for each file.'
Write-Host 'Images/text open directly. PDF/video need the preparation tools for native viewing.'
