$ErrorActionPreference = 'Stop'
Write-Host 'XEVU / REMOVABLE MEDIA INSTALLER' -ForegroundColor Green
Write-Host 'A selected device will be overwritten. No drive is preselected.'
& (Join-Path $PSScriptRoot 'write-media.ps1')
$xevuSelection = Read-Host 'Enter the disk number of your empty USB/SD card, or leave blank to exit'
if ([string]::IsNullOrWhiteSpace($xevuSelection)) { return }
$xevuNumber = 0
if (-not [int]::TryParse($xevuSelection,[ref]$xevuNumber)) { throw 'Invalid disk number.' }
$xevuChosen = Get-Disk -Number $xevuNumber
if (([string]$xevuChosen.SerialNumber).Trim()) {
    & (Join-Path $PSScriptRoot 'write-media.ps1') -DiskNumber $xevuNumber -ExpectedSerial $xevuChosen.SerialNumber
} else {
    & (Join-Path $PSScriptRoot 'write-media.ps1') -DiskNumber $xevuNumber -ExpectedUniqueId $xevuChosen.UniqueId
}
