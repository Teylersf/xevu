param([int]$MemoryMiB = 4096)
$ErrorActionPreference = 'Stop'
$nullgravVmRoot = Split-Path -Parent $PSScriptRoot
$nullgravVmImage = Join-Path $nullgravVmRoot 'dist\nullgrav-0.4.0-x86_64.img'
if (-not (Test-Path -LiteralPath $nullgravVmImage)) { throw 'Build or extract the NULLGRAV image into dist first.' }
$nullgravVmHashFile = [System.IO.Path]::ChangeExtension($nullgravVmImage,'.sha256')
if (-not (Test-Path -LiteralPath $nullgravVmHashFile)) { throw 'The image checksum file is missing.' }
$nullgravVmExpectedHash = ((Get-Content -LiteralPath $nullgravVmHashFile -Raw).Trim() -split '\s+')[0]
if ((Get-FileHash -LiteralPath $nullgravVmImage -Algorithm SHA256).Hash -ine $nullgravVmExpectedHash) { throw 'Image checksum mismatch.' }
$nullgravVmWindowsPath = $nullgravVmRoot.Replace('\','/')
$nullgravVmLinuxOutput = & wsl.exe -d Ubuntu -- wslpath -a $nullgravVmWindowsPath
if ($LASTEXITCODE -ne 0 -or -not $nullgravVmLinuxOutput) { throw 'Ubuntu WSL is required for this launcher.' }
$nullgravVmLinuxRoot = ([string]$nullgravVmLinuxOutput).Trim()
if (-not (Test-Path -LiteralPath (Join-Path $nullgravVmRoot 'test-output\interactive-vm-0.4.0\boot-test.img')) -and
    (Test-Path -LiteralPath (Join-Path $nullgravVmRoot 'test-output\interactive-vm-0.3.0\boot-test.img'))) {
    throw 'An earlier VM vault exists. Press Q in that VM, then run UPGRADE NULLGRAV VM.cmd to preserve its files and retry count.'
}
Write-Host 'NULLGRAV / INTERACTIVE VIRTUAL MACHINE' -ForegroundColor Green
Write-Host 'Only virtual disk images are attached. The physical SD card is not passed through.'
Write-Host 'Click inside the QEMU window to type. Press N to create a test vault; 1-6 navigate; Q shuts down.'
Write-Host 'WARNING: three failed unlocks erase this version''s virtual boot card. A correct unlock resets the count.' -ForegroundColor Yellow
Write-Host 'The interactive virtual disks persist under test-output\interactive-vm-0.4.0.'
$nullgravVmAction = if (Test-Path -LiteralPath (Join-Path $nullgravVmRoot 'test-output\interactive-vm-0.4.0\boot-test.img')) { 'resume' } else { 'run' }
& wsl.exe -d Ubuntu -- python3 ($nullgravVmLinuxRoot + '/tools/qemu.py') $nullgravVmAction --display gtk --memory-mib $MemoryMiB --session interactive
if ($LASTEXITCODE -ne 0) { throw 'The VM did not start or exited with an error. Check test-output\interactive-vm-0.4.0\qemu.log.' }
