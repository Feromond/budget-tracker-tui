@powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "%~dp0sign.ps1" %*
@exit /b %ERRORLEVEL%
