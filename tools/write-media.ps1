param([int]$DiskNumber = -1, [string]$ExpectedSerial, [string]$ExpectedUniqueId, [string]$Image = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist\xevu-0.5.0-x86_64.img'))
$ErrorActionPreference = 'Stop'
# Listing is always the default. A USB bus is not proof a device is disposable.
if ($DiskNumber -lt 0) {
    Get-Disk | Select-Object Number,FriendlyName,SerialNumber,UniqueId,BusType,Size,IsBoot,IsSystem
    Write-Host 'No device was opened or modified. Select an empty USB/SD card explicitly.'
    return
}
$xevuDisk = Get-Disk -Number $DiskNumber
if ($xevuDisk.IsBoot -or $xevuDisk.IsSystem) { throw 'Refusing a Windows boot/system disk.' }
if ($xevuDisk.BusType -notin @('USB','SD','MMC')) { throw 'Only explicitly identified removable USB/SD/MMC media are allowed.' }
if ($ExpectedSerial) {
    if (([string]$xevuDisk.SerialNumber).Trim() -ne $ExpectedSerial.Trim()) { throw 'The explicit serial number must match the selected disk.' }
} elseif (-not $ExpectedUniqueId -or $xevuDisk.UniqueId -ne $ExpectedUniqueId) { throw 'Supply the exact serial or, for a reader without a serial, its explicit UniqueId.' }
if ($xevuDisk.LogicalSectorSize -ne 512) { throw 'This installer currently supports 512-byte logical sectors only.' }
$xevuPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $xevuPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Run the selected-device write in an elevated PowerShell window.' }
$xevuImagePath = (Resolve-Path -LiteralPath $Image).Path
$xevuImageFile = Get-Item -LiteralPath $xevuImagePath
if ($xevuDisk.Size -lt $xevuImageFile.Length -or $xevuDisk.Size -gt 128GB) { throw 'Destination must fit the image and be at most 128 GB.' }
$xevuHashPath = [System.IO.Path]::ChangeExtension($xevuImagePath,'.sha256')
if (-not (Test-Path -LiteralPath $xevuHashPath)) { throw 'Required image checksum file is missing.' }
$xevuExpectedHash = ((Get-Content -LiteralPath $xevuHashPath -Raw).Trim() -split '\s+')[0].ToLowerInvariant()
if ((Get-FileHash -LiteralPath $xevuImagePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $xevuExpectedHash) { throw 'Image checksum mismatch.' }
Write-Host ('ERASE DISK {0}: {1}, {2:N2} GB, serial {3}' -f $DiskNumber,$xevuDisk.FriendlyName,($xevuDisk.Size/1GB),$xevuDisk.SerialNumber)
$xevuPhrase = Read-Host ('Type ERASE {0} to overwrite this selected device' -f $DiskNumber)
if ($xevuPhrase -cne ('ERASE {0}' -f $DiskNumber)) { throw 'No write authorized.' }
$xevuRecheck = Get-Disk -Number $DiskNumber
if ($xevuRecheck.IsBoot -or $xevuRecheck.IsSystem -or $xevuRecheck.Size -ne $xevuDisk.Size -or $xevuRecheck.SerialNumber -ne $xevuDisk.SerialNumber -or $xevuRecheck.UniqueId -ne $xevuDisk.UniqueId) { throw 'Device identity changed; no writes.' }
if (-not ('XevuVolumeLock' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class XevuVolumeLock {
    [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true,EntryPoint="CreateFileW")]
    static extern SafeFileHandle Open(string path,uint access,uint share,IntPtr security,uint mode,uint flags,IntPtr template);
    [DllImport("kernel32.dll",SetLastError=true)]
    static extern bool DeviceIoControl(SafeFileHandle h,uint code,IntPtr input,uint inSize,IntPtr output,uint outSize,out uint returned,IntPtr overlapped);
    public static SafeFileHandle Lock(string path) {
        var h=Open(path,0xC0000000,3,IntPtr.Zero,3,0,IntPtr.Zero);
        if(h.IsInvalid)throw new Win32Exception(Marshal.GetLastWin32Error());
        uint n;
        if(!DeviceIoControl(h,0x90018,IntPtr.Zero,0,IntPtr.Zero,0,out n,IntPtr.Zero)) { var e=Marshal.GetLastWin32Error();h.Dispose();throw new Win32Exception(e); }
        if(!DeviceIoControl(h,0x90020,IntPtr.Zero,0,IntPtr.Zero,0,out n,IntPtr.Zero)) { var e=Marshal.GetLastWin32Error();h.Dispose();throw new Win32Exception(e); }
        return h;
    }
}
'@
}
$xevuDevicePath = '\\.\PhysicalDrive' + $DiskNumber
$xevuInput = [System.IO.File]::OpenRead($xevuImagePath)
$xevuOutput = $null
$xevuLocks = New-Object 'System.Collections.Generic.List[Microsoft.Win32.SafeHandles.SafeFileHandle]'
try {
    foreach ($xevuPartition in @(Get-Partition -DiskNumber $DiskNumber -ErrorAction SilentlyContinue)) {
        foreach ($xevuVolumePath in @($xevuPartition.AccessPaths | Where-Object { $_ -like '\\?\Volume*' })) {
            $xevuLocks.Add([XevuVolumeLock]::Lock($xevuVolumePath.TrimEnd('\')))
        }
    }
    $xevuOutput = [System.IO.File]::Open($xevuDevicePath,[System.IO.FileMode]::Open,[System.IO.FileAccess]::ReadWrite,[System.IO.FileShare]::ReadWrite)
    $xevuInput.CopyTo($xevuOutput,1048576)
    $xevuOutput.Flush($true)
    $xevuOutput.Position = 0
    $xevuInput.Position = 0
    $xevuBuffer = New-Object byte[] 1048576
    $xevuReadback = [System.Security.Cryptography.IncrementalHash]::CreateHash([System.Security.Cryptography.HashAlgorithmName]::SHA256)
    $xevuRemaining = $xevuImageFile.Length
    while ($xevuRemaining -gt 0) {
        $xevuRead = $xevuOutput.Read($xevuBuffer,0,[int][Math]::Min($xevuBuffer.Length,$xevuRemaining))
        if ($xevuRead -le 0) { throw 'Unexpected end of device during verification.' }
        $xevuReadback.AppendData($xevuBuffer,0,$xevuRead)
        $xevuRemaining -= $xevuRead
    }
    $xevuActualHash = [BitConverter]::ToString($xevuReadback.GetHashAndReset()).Replace('-','').ToLowerInvariant()
    $xevuReadback.Dispose()
    if ($xevuActualHash -ne $xevuExpectedHash) { throw 'DEVICE VERIFICATION FAILED. Do not boot this copy.' }
    Write-Host ('Verified image bytes on selected device: SHA256 ' + $xevuActualHash)
    Write-Host 'Safely eject it. Existing data beyond the image was not sanitized. The full physical card was not erased.'
} finally { if ($xevuOutput) { $xevuOutput.Dispose() }; foreach ($xevuLock in $xevuLocks) { $xevuLock.Dispose() }; $xevuInput.Dispose() }
