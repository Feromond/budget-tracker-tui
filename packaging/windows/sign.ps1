param([Parameter(Mandatory)][string]$File)
$ErrorActionPreference = 'Stop'

foreach ($name in 'ARTIFACT_SIGNING_ACCOUNT', 'ARTIFACT_SIGNING_ENDPOINT', 'ARTIFACT_SIGNING_PROFILE', 'ARTIFACT_SIGNING_DLIB') {
    if (-not [Environment]::GetEnvironmentVariable($name)) { throw "$name isn't set" }
}

$signtool = $env:SIGNTOOL
if (-not $signtool) {
    $signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" |
        Sort-Object { try { [version]$_.Directory.Parent.Name } catch { [version]'0.0' } } |
        Select-Object -Last 1 -ExpandProperty FullName
}
if (-not $signtool) { throw 'signtool.exe not found; install the Windows SDK' }

$metadata = @{
    Endpoint               = $env:ARTIFACT_SIGNING_ENDPOINT
    CodeSigningAccountName = $env:ARTIFACT_SIGNING_ACCOUNT
    CertificateProfileName = $env:ARTIFACT_SIGNING_PROFILE
}
# Fail on bad credentials instead of trying other logins.
if ($env:AZURE_CLIENT_SECRET) {
    $metadata.ExcludeCredentials = @(
        'ManagedIdentityCredential', 'WorkloadIdentityCredential', 'SharedTokenCacheCredential',
        'VisualStudioCredential', 'VisualStudioCodeCredential', 'AzureCliCredential',
        'AzurePowerShellCredential', 'AzureDeveloperCliCredential', 'InteractiveBrowserCredential'
    )
}
$metadataPath = Join-Path ([IO.Path]::GetTempPath()) "budget-tracker-signing-$PID.json"
$metadata | ConvertTo-Json | Set-Content -Path $metadataPath -Encoding utf8

try {
    # Timestamp so signatures outlast the three-day certificate.
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        & $signtool sign /v /fd SHA256 /td SHA256 /tr 'http://timestamp.acs.microsoft.com' `
            /dlib $env:ARTIFACT_SIGNING_DLIB /dmdf $metadataPath $File
        if ($LASTEXITCODE -eq 0) { break }
        if ($attempt -eq 3) { throw "signing $File failed" }
        Write-Warning "signing $File failed; retrying"
        Start-Sleep -Seconds 10
    }
} finally {
    Remove-Item $metadataPath -ErrorAction SilentlyContinue
}

& $signtool verify /pa $File
if ($LASTEXITCODE -ne 0) { throw "$File didn't verify after signing" }
if ($env:SIGNED_LOG) { Add-Content -Path $env:SIGNED_LOG -Value $File }
