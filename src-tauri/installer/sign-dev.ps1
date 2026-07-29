# Dev-only self-signed code signing for TCursor.
#
# WARNING: a self-signed certificate removes the "Unknown Publisher" warning ONLY on machines
# that explicitly trust it. It does NOT help end users who download the app - that needs a real
# OV/EV certificate. See SIGNING.md.
#
# Usage:  ./sign-dev.ps1        (run from src-tauri/installer in PowerShell)

$ErrorActionPreference = "Stop"

$subject = "CN=Coehe, O=Coehe"
$cerPath = Join-Path $PSScriptRoot "tcursor-dev.cer"

Write-Host "Creating a self-signed code-signing certificate ($subject)..."
$cert = New-SelfSignedCertificate `
  -Type CodeSigningCert `
  -Subject $subject `
  -CertStoreLocation "Cert:\CurrentUser\My" `
  -NotAfter (Get-Date).AddYears(3) `
  -KeyExportPolicy Exportable `
  -KeySpec Signature

# Export the public cert (for optionally trusting it on this machine).
Export-Certificate -Cert $cert -FilePath $cerPath | Out-Null

Write-Host ""
Write-Host "Certificate created." -ForegroundColor Green
Write-Host "  Thumbprint : $($cert.Thumbprint)"
Write-Host "  Public cert: $cerPath"
Write-Host ""
Write-Host "1) Sign the build automatically: put this in tauri.conf.json -> bundle.windows:"
Write-Host "     `"certificateThumbprint`": `"$($cert.Thumbprint)`","
Write-Host "     `"digestAlgorithm`": `"sha256`","
Write-Host "     `"timestampUrl`": `"http://timestamp.digicert.com`""
Write-Host "   then:  npm run tauri build"
Write-Host ""
Write-Host "2) OPTIONAL - trust this cert so the warning disappears ON THIS MACHINE."
Write-Host "   This changes your certificate trust; run it deliberately, not blindly:"
Write-Host "     Import-Certificate -FilePath `"$cerPath`" -CertStoreLocation Cert:\CurrentUser\Root"
Write-Host "     Import-Certificate -FilePath `"$cerPath`" -CertStoreLocation Cert:\CurrentUser\TrustedPublisher"
Write-Host ""
Write-Host "Do NOT commit the thumbprint, tcursor-dev.cer, or any .pfx." -ForegroundColor Yellow
