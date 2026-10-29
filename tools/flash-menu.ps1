$ErrorActionPreference = 'Stop'
Write-Host 'NULLGRAV / REMOVABLE MEDIA INSTALLER' -ForegroundColor Green
Write-Host 'A selected device will be overwritten. No drive is preselected.'
& (Join-Path $PSScriptRoot 'write-media.ps1')
$nullgravSelection = Read-Host 'Enter the disk number of your empty USB/SD card, or leave blank to exit'
if ([string]::IsNullOrWhiteSpace($nullgravSelection)) { return }
$nullgravNumber = 0
if (-not [int]::TryParse($nullgravSelection,[ref]$nullgravNumber)) { throw 'Invalid disk number.' }
$nullgravChosen = Get-Disk -Number $nullgravNumber
if (([string]$nullgravChosen.SerialNumber).Trim()) {
    & (Join-Path $PSScriptRoot 'write-media.ps1') -DiskNumber $nullgravNumber -ExpectedSerial $nullgravChosen.SerialNumber
} else {
    & (Join-Path $PSScriptRoot 'write-media.ps1') -DiskNumber $nullgravNumber -ExpectedUniqueId $nullgravChosen.UniqueId
}
