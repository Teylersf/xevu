param([int]$MemoryMiB = 4096)
$ErrorActionPreference = 'Stop'
$xevuVmRoot = Split-Path -Parent $PSScriptRoot
$xevuVmImage = Join-Path $xevuVmRoot 'dist\xevu-0.5.0-x86_64.img'
if (-not (Test-Path -LiteralPath $xevuVmImage)) { throw 'Build or extract the XEVU image into dist first.' }
$xevuVmHashFile = [System.IO.Path]::ChangeExtension($xevuVmImage,'.sha256')
if (-not (Test-Path -LiteralPath $xevuVmHashFile)) { throw 'The image checksum file is missing.' }
$xevuVmExpectedHash = ((Get-Content -LiteralPath $xevuVmHashFile -Raw).Trim() -split '\s+')[0]
if ((Get-FileHash -LiteralPath $xevuVmImage -Algorithm SHA256).Hash -ine $xevuVmExpectedHash) { throw 'Image checksum mismatch.' }
$xevuVmWindowsPath = $xevuVmRoot.Replace('\','/')
$xevuVmLinuxOutput = & wsl.exe -d Ubuntu -- wslpath -a $xevuVmWindowsPath
if ($LASTEXITCODE -ne 0 -or -not $xevuVmLinuxOutput) { throw 'Ubuntu WSL is required for this launcher.' }
$xevuVmLinuxRoot = ([string]$xevuVmLinuxOutput).Trim()
if (-not (Test-Path -LiteralPath (Join-Path $xevuVmRoot 'test-output\interactive-vm-0.5.0\boot-test.img')) -and
    ((Test-Path -LiteralPath (Join-Path $xevuVmRoot 'test-output\interactive-vm-0.4.0\boot-test.img')) -or
     (Test-Path -LiteralPath (Join-Path $xevuVmRoot 'test-output\interactive-vm-0.3.0\boot-test.img')))) {
    throw 'An earlier VM vault exists. Press Q in that VM, then run UPGRADE XEVU VM.cmd to preserve its files and retry count.'
}
Write-Host 'XEVU / INTERACTIVE VIRTUAL MACHINE' -ForegroundColor Green
Write-Host 'Only virtual disk images are attached. The physical SD card is not passed through.'
Write-Host 'Click inside the QEMU window to type. Press N to create a test vault; 1-6 navigate; Q shuts down.'
Write-Host 'WARNING: three failed unlocks erase this version''s virtual boot card. A correct unlock resets the count.' -ForegroundColor Yellow
Write-Host 'The interactive virtual disks persist under test-output\interactive-vm-0.5.0.'
$xevuVmAction = if (Test-Path -LiteralPath (Join-Path $xevuVmRoot 'test-output\interactive-vm-0.5.0\boot-test.img')) { 'resume' } else { 'run' }
& wsl.exe -d Ubuntu -- python3 ($xevuVmLinuxRoot + '/tools/qemu.py') $xevuVmAction --display gtk --memory-mib $MemoryMiB --session interactive
if ($LASTEXITCODE -ne 0) { throw 'The VM did not start or exited with an error. Check test-output\interactive-vm-0.5.0\qemu.log.' }
