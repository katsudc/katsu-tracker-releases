param(
 [Parameter(Mandatory=$true)][string]$PackagePath,
 [Parameter(Mandatory=$true)][string]$CertificatePath,
 [Parameter(Mandatory=$true)][ValidatePattern('^[a-fA-F0-9]{40}$')][string]$ExpectedCertificateThumbprint,
 [switch]$TrustMachinePublisher,
 [switch]$CertificateOnly
)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -AssemblyName System.Security
$PackagePath=(Resolve-Path -LiteralPath $PackagePath).Path
$CertificatePath=(Resolve-Path -LiteralPath $CertificatePath).Path
$certificate=[System.Security.Cryptography.X509Certificates.X509Certificate2]::new($CertificatePath)
if($certificate.Subject -ne 'CN=katsu' -or $certificate.Thumbprint -ne $ExpectedCertificateThumbprint){throw 'Unexpected widget publisher'}
$archive=[IO.Compression.ZipFile]::OpenRead($PackagePath)
try {
 $entry=$archive.GetEntry('AppxSignature.p7x')
 if(-not $entry){throw 'Unsigned widget package'}
 $stream=$entry.Open();$memory=[IO.MemoryStream]::new();$stream.CopyTo($memory);$stream.Dispose()
 $bytes=$memory.ToArray();$memory.Dispose()
 if($bytes.Length -lt 5 -or [Text.Encoding]::ASCII.GetString($bytes,0,4) -ne 'PKCX'){throw 'Invalid package signature'}
 $cms=[Security.Cryptography.Pkcs.SignedCms]::new();$cms.Decode($bytes[4..($bytes.Length-1)]);$cms.CheckSignature($true)
 if($cms.SignerInfos.Count -ne 1 -or $cms.SignerInfos[0].Certificate.Thumbprint -ne $certificate.Thumbprint){throw 'Widget publisher does not match bundled certificate'}
} finally {$archive.Dispose()}
$trusted=@(Get-ChildItem Cert:\LocalMachine\TrustedPeople -ErrorAction SilentlyContinue | Where-Object {$_.Thumbprint -eq $certificate.Thumbprint}).Count -gt 0
if(-not $trusted){
 if(-not $TrustMachinePublisher){Write-Output 'publisher_trust_required';exit 3}
 # Local MSIX deployment requires LocalMachine trust. This switch is passed
 # only after explicit consent covering all accounts on this computer.
 # Never use Trusted Root or change Developer Mode / capture settings.
 $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
 if(-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){
  # Elevate only certificate registration. Package registration below stays in
  # the original user's process, including when UAC uses another admin account.
  $quote={param([string]$value) "'"+$value.Replace("'","''")+"'"}
  $code='& '+(& $quote $PSCommandPath)+' -PackagePath '+(& $quote $PackagePath)+' -CertificatePath '+(& $quote $CertificatePath)+' -ExpectedCertificateThumbprint '+(& $quote $ExpectedCertificateThumbprint)+' -TrustMachinePublisher -CertificateOnly; exit $LASTEXITCODE'
  $encoded=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($code))
  $shell=Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'
  try {$child=Start-Process -FilePath $shell -ArgumentList @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-EncodedCommand',$encoded) -Verb RunAs -WindowStyle Hidden -Wait -PassThru -ErrorAction Stop}
  catch {
   # Start-Process can wrap Win32 ERROR_CANCELLED in an InvalidOperationException.
   for($cause=$_.Exception;$null -ne $cause;$cause=$cause.InnerException){
    if($cause -is [ComponentModel.Win32Exception] -and $cause.NativeErrorCode -eq 1223){Write-Output 'setup_cancelled';exit 5}
   }
   Write-Output 'administrator_required';exit 4
  }
  if($child.ExitCode -ne 0){exit $child.ExitCode}
  $trusted=@(Get-ChildItem Cert:\LocalMachine\TrustedPeople -ErrorAction SilentlyContinue | Where-Object {$_.Thumbprint -eq $ExpectedCertificateThumbprint}).Count -gt 0
  if(-not $trusted){Write-Output 'administrator_required';exit 4}
 }else{
 $store=[Security.Cryptography.X509Certificates.X509Store]::new('TrustedPeople','LocalMachine')
 try {$store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite);$store.Add($certificate)}finally{$store.Close()}
 }
}
if($CertificateOnly){Write-Output 'publisher_trusted';exit 0}
Add-AppxPackage -Path $PackagePath -ForceUpdateFromAnyVersion -ForceApplicationShutdown -ErrorAction Stop
Write-Output 'widget_installed'
