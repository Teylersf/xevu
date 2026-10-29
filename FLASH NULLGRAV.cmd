@echo off
cd /d "%~dp0"
echo NULLGRAV - run this launcher as Administrator for physical-media writes.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0tools\flash-menu.ps1"
pause
