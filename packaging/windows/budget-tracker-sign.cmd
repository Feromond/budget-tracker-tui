@setlocal
@rem makensis runs this by bare name from PATH, where %~dp0 resolves to the current
@rem folder instead of this one, so CI passes the script path explicitly.
@set "script=%BUDGET_TRACKER_SIGN_SCRIPT%"
@if not defined script set "script=%~dp0sign.ps1"
@powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "%script%" %*
@exit /b %ERRORLEVEL%
