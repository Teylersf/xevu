param([int]$DiskNumber = -1, [string]$ExpectedSerial, [string]$ExpectedUniqueId, [string]$Image = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist\nullgrav-0.1.0-x86_64.img'))
$ErrorActionPreference = 'Stop'
# Listing is always the default. A USB bus is not proof a device is disposable.
if ($DiskNumber -lt 0) {
    Get-Disk | Select-Object Number,FriendlyName,SerialNumber,UniqueId,BusType,Size,IsBoot,IsSystem
    Write-Host 'No device was opened or modified. Select an empty USB/SD card explicitly.'
    return
}
$nullgravDisk = Get-Disk -Number $DiskNumber
if ($nullgravDisk.IsBoot -or $nullgravDisk.IsSystem) { throw 'Refusing a Windows boot/system disk.' }
if ($nullgravDisk.BusType -notin @('USB','SD','MMC')) { throw 'Only explicitly identified removable USB/SD/MMC media are allowed.' }
if ($ExpectedSerial) {
    if (([string]$nullgravDisk.SerialNumber).Trim() -ne $ExpectedSerial.Trim()) { throw 'The explicit serial number must match the selected disk.' }
} elseif (-not $ExpectedUniqueId -or $nullgravDisk.UniqueId -ne $ExpectedUniqueId) { throw 'Supply the exact serial or, for a reader without a serial, its explicit UniqueId.' }
if ($nullgravDisk.LogicalSectorSize -ne 512) { throw 'This installer currently supports 512-byte logical sectors only.' }
$nullgravPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $nullgravPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Run the selected-device write in an elevated PowerShell window.' }
$nullgravImagePath = (Resolve-Path -LiteralPath $Image).Path
$nullgravImageFile = Get-Item -LiteralPath $nullgravImagePath
if ($nullgravDisk.Size -lt $nullgravImageFile.Length -or $nullgravDisk.Size -gt 128GB) { throw 'Destination must fit the image and be at most 128 GB.' }
$nullgravHashPath = [System.IO.Path]::ChangeExtension($nullgravImagePath,'.sha256')
if (-not (Test-Path -LiteralPath $nullgravHashPath)) { throw 'Required image checksum file is missing.' }
$nullgravExpectedHash = ((Get-Content -LiteralPath $nullgravHashPath -Raw).Trim() -split '\s+')[0].ToLowerInvariant()
if ((Get-FileHash -LiteralPath $nullgravImagePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $nullgravExpectedHash) { throw 'Image checksum mismatch.' }
Write-Host ('ERASE DISK {0}: {1}, {2:N2} GB, serial {3}' -f $DiskNumber,$nullgravDisk.FriendlyName,($nullgravDisk.Size/1GB),$nullgravDisk.SerialNumber)
$nullgravPhrase = Read-Host ('Type ERASE {0} to overwrite this selected device' -f $DiskNumber)
if ($nullgravPhrase -cne ('ERASE {0}' -f $DiskNumber)) { throw 'No write authorized.' }
$nullgravRecheck = Get-Disk -Number $DiskNumber
if ($nullgravRecheck.IsBoot -or $nullgravRecheck.IsSystem -or $nullgravRecheck.Size -ne $nullgravDisk.Size -or $nullgravRecheck.SerialNumber -ne $nullgravDisk.SerialNumber -or $nullgravRecheck.UniqueId -ne $nullgravDisk.UniqueId) { throw 'Device identity changed; no writes.' }
if (-not ('NullgravVolumeLock' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class NullgravVolumeLock {
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
$nullgravDevicePath = '\\.\PhysicalDrive' + $DiskNumber
$nullgravInput = [System.IO.File]::OpenRead($nullgravImagePath)
$nullgravOutput = $null
$nullgravLocks = New-Object 'System.Collections.Generic.List[Microsoft.Win32.SafeHandles.SafeFileHandle]'
try {
    foreach ($nullgravPartition in @(Get-Partition -DiskNumber $DiskNumber -ErrorAction SilentlyContinue)) {
        foreach ($nullgravVolumePath in @($nullgravPartition.AccessPaths | Where-Object { $_ -like '\\?\Volume*' })) {
            $nullgravLocks.Add([NullgravVolumeLock]::Lock($nullgravVolumePath.TrimEnd('\')))
        }
    }
    $nullgravOutput = [System.IO.File]::Open($nullgravDevicePath,[System.IO.FileMode]::Open,[System.IO.FileAccess]::ReadWrite,[System.IO.FileShare]::ReadWrite)
    $nullgravInput.CopyTo($nullgravOutput,1048576)
    $nullgravOutput.Flush($true)
    $nullgravOutput.Position = 0
    $nullgravInput.Position = 0
    $nullgravBuffer = New-Object byte[] 1048576
    $nullgravReadback = [System.Security.Cryptography.IncrementalHash]::CreateHash([System.Security.Cryptography.HashAlgorithmName]::SHA256)
    $nullgravRemaining = $nullgravImageFile.Length
    while ($nullgravRemaining -gt 0) {
        $nullgravRead = $nullgravOutput.Read($nullgravBuffer,0,[int][Math]::Min($nullgravBuffer.Length,$nullgravRemaining))
        if ($nullgravRead -le 0) { throw 'Unexpected end of device during verification.' }
        $nullgravReadback.AppendData($nullgravBuffer,0,$nullgravRead)
        $nullgravRemaining -= $nullgravRead
    }
    $nullgravActualHash = [BitConverter]::ToString($nullgravReadback.GetHashAndReset()).Replace('-','').ToLowerInvariant()
    $nullgravReadback.Dispose()
    if ($nullgravActualHash -ne $nullgravExpectedHash) { throw 'DEVICE VERIFICATION FAILED. Do not boot this copy.' }
    Write-Host ('Verified image bytes on selected device: SHA256 ' + $nullgravActualHash)
    Write-Host 'Safely eject it. Existing data beyond the image was not sanitized. The full physical card was not erased.'
} finally { if ($nullgravOutput) { $nullgravOutput.Dispose() }; foreach ($nullgravLock in $nullgravLocks) { $nullgravLock.Dispose() }; $nullgravInput.Dispose() }
