param([string]$Destination, [string]$SessionDirectory)
$ErrorActionPreference = 'Stop'
if (-not $Destination) {
    Add-Type -AssemblyName System.Windows.Forms
    $nullgravExportDialog = New-Object System.Windows.Forms.FolderBrowserDialog
    $nullgravExportDialog.Description = 'Choose the Windows folder for normal decrypted files (for example a folder on C:)'
    $nullgravExportDialog.ShowNewFolderButton = $true
    try {
        if ($nullgravExportDialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) { exit 0 }
        $Destination = $nullgravExportDialog.SelectedPath
    } finally { $nullgravExportDialog.Dispose() }
}
Write-Host 'NULLGRAV / SAVE ORIGINAL FILES TO WINDOWS' -ForegroundColor Green
Write-Host ('Destination: ' + $Destination)
Write-Host 'Exported files are normal UNENCRYPTED copies. Existing files are not overwritten.' -ForegroundColor Yellow
Write-Host 'Keep this window open. After exporting inside NULLGRAV, press Q in the VM.'
$nullgravExportArguments = @((Join-Path $PSScriptRoot 'vm_export.py'),'run','--destination',$Destination)
if ($SessionDirectory) { $nullgravExportArguments += @('--session-dir',$SessionDirectory) }
& python @nullgravExportArguments
if ($LASTEXITCODE -ne 0) { throw 'Export did not complete. See the message above.' }
