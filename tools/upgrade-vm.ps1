$ErrorActionPreference = 'Stop'
$xevuUpgradeRoot = Split-Path -Parent $PSScriptRoot
$xevuUpgradeLinuxRoot = (& wsl.exe -d Ubuntu -- wslpath -a $xevuUpgradeRoot.Replace('\','/')).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Ubuntu WSL is required.' }
Write-Host 'XEVU / UPGRADE THE VM AND KEEP ITS ENCRYPTED VAULT' -ForegroundColor Green
Write-Host 'Close any passphrase prompt with Esc, then press Q in the old VM when ready.'
Write-Host 'Its encrypted vault and retry count are copied; existing disks are retained as a backup.'
& wsl.exe -d Ubuntu -- python3 ($xevuUpgradeLinuxRoot + '/tools/upgrade_vm.py') --wait --launch
if ($LASTEXITCODE -ne 0) { throw 'Upgrade did not complete; the original VM disks were kept.' }
