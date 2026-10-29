param([string]$Destination, [string]$SessionDirectory)
$ErrorActionPreference = 'Stop'
if (-not $Destination) {
    Add-Type -AssemblyName System.Windows.Forms
    $xevuExportDialog = New-Object System.Windows.Forms.FolderBrowserDialog
    $xevuExportDialog.Description = 'Choose the Windows folder for normal decrypted files (for example a folder on C:)'
    $xevuExportDialog.ShowNewFolderButton = $true
    try {
        if ($xevuExportDialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) { exit 0 }
        $Destination = $xevuExportDialog.SelectedPath
    } finally { $xevuExportDialog.Dispose() }
}
Write-Host 'XEVU / SAVE ORIGINAL FILES TO WINDOWS' -ForegroundColor Green
Write-Host ('Destination: ' + $Destination)
Write-Host 'Exported files are normal UNENCRYPTED copies. Existing files are not overwritten.' -ForegroundColor Yellow
Write-Host 'Keep this window open. After exporting inside XEVU, press Q in the VM.'
$xevuExportArguments = @((Join-Path $PSScriptRoot 'vm_export.py'),'run','--destination',$Destination)
if ($SessionDirectory) { $xevuExportArguments += @('--session-dir',$SessionDirectory) }
& python @nullgravExportArguments
if ($LASTEXITCODE -ne 0) { throw 'Export did not complete. See the message above.' }
