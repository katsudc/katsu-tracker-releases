param(
 [Parameter(Mandatory=$true)][string]$PackagePath,
 [Parameter(Mandatory=$true)][string]$CertificatePath,
 [Parameter(Mandatory=$true)][ValidatePattern('^[a-fA-F0-9]{40}$')][string]$ExpectedCertificateThumbprint,
 [string]$DependencyPath='',
 [switch]$TrustMachinePublisher,
 [switch]$CertificateOnly,
 [switch]$DiagnoseOnly
)
$ErrorActionPreference='Stop'
$script:setupStage='preflight'
$script:runtimePresent=$null
$script:publisherTrusted=$null
function Complete-Setup([string]$Code,[int]$ExitCode,[string]$HResult='',[string]$ActivityId='') {
 $report=[ordered]@{schema=1;stage=$script:setupStage;code=$Code;hresult=$null;activity_id=$null;runtime_present=$script:runtimePresent;publisher_trusted=$script:publisherTrusted}
 if($HResult -match '^0x[0-9a-fA-F]{8}$'){$report.hresult=$HResult}
 if($ActivityId -match '^[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}$'){$report.activity_id=$ActivityId}
 Write-Output ('KATSU_WIDGET_SETUP:'+($report|ConvertTo-Json -Compress))
 exit $ExitCode
}
try {
 if($PSVersionTable.PSEdition -ne 'Desktop'){Complete-Setup 'powershell_unavailable' 1}
 if([Environment]::OSVersion.Version.Build -lt 18362){Complete-Setup 'windows_unsupported' 1}
 $script:setupStage='validate_package'
 Add-Type -AssemblyName System.IO.Compression.FileSystem
 Add-Type -AssemblyName System.Security
 $PackagePath=(Resolve-Path -LiteralPath $PackagePath).Path
 $CertificatePath=(Resolve-Path -LiteralPath $CertificatePath).Path
 $certificate=[System.Security.Cryptography.X509Certificates.X509Certificate2]::new($CertificatePath)
 if($certificate.Subject -ne 'CN=katsu' -or $certificate.Thumbprint -ne $ExpectedCertificateThumbprint){Complete-Setup 'publisher_mismatch' 1}
 if((Get-Date) -lt $certificate.NotBefore -or (Get-Date) -gt $certificate.NotAfter){Complete-Setup 'certificate_expired' 1}
 $archive=[IO.Compression.ZipFile]::OpenRead($PackagePath)
 try {
  $entry=$archive.GetEntry('AppxSignature.p7x')
  if(-not $entry){Complete-Setup 'package_signature_invalid' 1}
  $stream=$entry.Open();$memory=[IO.MemoryStream]::new();$stream.CopyTo($memory);$stream.Dispose()
  $bytes=$memory.ToArray();$memory.Dispose()
  if($bytes.Length -lt 5 -or [Text.Encoding]::ASCII.GetString($bytes,0,4) -ne 'PKCX'){Complete-Setup 'package_signature_invalid' 1}
  $cms=[Security.Cryptography.Pkcs.SignedCms]::new();$cms.Decode($bytes[4..($bytes.Length-1)]);$cms.CheckSignature($true)
  if($cms.SignerInfos.Count -ne 1 -or $cms.SignerInfos[0].Certificate.Thumbprint -ne $certificate.Thumbprint){Complete-Setup 'publisher_mismatch' 1}
 } finally {$archive.Dispose()}
 $script:publisherTrusted=@(Get-ChildItem Cert:\LocalMachine\TrustedPeople -ErrorAction SilentlyContinue|Where-Object {$_.Thumbprint -eq $certificate.Thumbprint}).Count -gt 0
 if(-not $CertificateOnly){
  $script:setupStage='dependency_check'
  # This C++/CX UWP widget needs the app-container framework, not UWPDesktop or the desktop VC redist.
  $null=Get-Command Get-AppxPackage -ErrorAction Stop
  $script:runtimePresent=@(Get-AppxPackage -Name 'Microsoft.VCLibs.140.00' -ErrorAction Stop|Where-Object {
   ([string]$_.Architecture -in @('X64','Neutral','9','11')) -and [version]$_.Version -ge [version]'14.0.0.0' -and $_.Publisher -eq 'CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US'
  }).Count -gt 0
  if(-not $script:runtimePresent){
   if(-not $DependencyPath -or -not(Test-Path -LiteralPath $DependencyPath)){Complete-Setup 'runtime_missing' 6}
   $DependencyPath=(Resolve-Path -LiteralPath $DependencyPath).Path
   if((Get-FileHash -LiteralPath $DependencyPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne '9c17b521f9d690a1f504da5108ed6eec5669eb3a8fd1331eef43e40d84e74283'){Complete-Setup 'dependency_package_invalid' 1}
  }
 }
 if($DiagnoseOnly){$script:setupStage='preflight';Complete-Setup 'preflight_ok' 0}
 $script:setupStage='publisher_trust'
 if(-not $script:publisherTrusted){
  if(-not $TrustMachinePublisher){Complete-Setup 'publisher_trust_required' 3}
  $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
  if(-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)){
   # Elevate only certificate registration. Keep package registration in the original account.
   $quote={param([string]$value) "'"+$value.Replace("'","''")+"'"}
   $code='& '+(& $quote $PSCommandPath)+' -PackagePath '+(& $quote $PackagePath)+' -CertificatePath '+(& $quote $CertificatePath)+' -ExpectedCertificateThumbprint '+(& $quote $ExpectedCertificateThumbprint)+' -TrustMachinePublisher -CertificateOnly; exit $LASTEXITCODE'
   $encoded=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($code))
   $shell=Join-Path $env:SystemRoot 'System32/WindowsPowerShell/v1.0/powershell.exe'
   try {$child=Start-Process -FilePath $shell -ArgumentList @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-EncodedCommand',$encoded) -Verb RunAs -WindowStyle Hidden -Wait -PassThru -ErrorAction Stop}
   catch {
    for($cause=$_.Exception;$null -ne $cause;$cause=$cause.InnerException){
     if($cause -is [ComponentModel.Win32Exception] -and $cause.NativeErrorCode -eq 1223){Complete-Setup 'setup_cancelled' 5}
    }
    Complete-Setup 'administrator_required' 4
   }
   if($child.ExitCode -ne 0){Complete-Setup 'certificate_registration_failed' ([int]$child.ExitCode)}
   $script:publisherTrusted=@(Get-ChildItem Cert:\LocalMachine\TrustedPeople -ErrorAction SilentlyContinue|Where-Object {$_.Thumbprint -eq $ExpectedCertificateThumbprint}).Count -gt 0
   if(-not $script:publisherTrusted){Complete-Setup 'administrator_required' 4}
  }else{
   $store=[Security.Cryptography.X509Certificates.X509Store]::new('TrustedPeople','LocalMachine')
   try {$store.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite);$store.Add($certificate)}finally{$store.Close()}
   $script:publisherTrusted=$true
  }
 }
 if($CertificateOnly){Complete-Setup 'publisher_trusted' 0}
 $script:setupStage='register_widget'
 if(-not $script:runtimePresent){
  Add-AppxPackage -Path $PackagePath -DependencyPath $DependencyPath -ForceUpdateFromAnyVersion -ForceApplicationShutdown -ErrorAction Stop
  $script:runtimePresent=$true
 }else{
  Add-AppxPackage -Path $PackagePath -ForceUpdateFromAnyVersion -ForceApplicationShutdown -ErrorAction Stop
 }
 $script:setupStage='ready'
 Complete-Setup 'widget_installed' 0
}catch{
 # Windows deployment messages can contain paths/account details. Export only numeric codes and this operation's ActivityId.
 $text=[string]$_.Exception.Message+' '+[string]$_.ErrorDetails.Message
 $match=[regex]::Match($text,'(?i)0x[0-9a-f]{8}')
 $hresult=if($match.Success){'0x'+$match.Value.Substring(2).ToUpperInvariant()}else{'0x'+([BitConverter]::ToUInt32([BitConverter]::GetBytes([int]$_.Exception.HResult),0)).ToString('X8')}
 $activity=[regex]::Match($text,'(?i)ActivityId[^0-9a-f]{0,12}([0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12})')
 $activityId=if($activity.Success){$activity.Groups[1].Value}else{''}
 $code=switch($hresult){
  '0x80073CF3' {'dependency_missing'}
  '0x80073CFF' {'deployment_policy'}
  '0x80073D01' {'deployment_policy'}
  '0x800B0109' {'publisher_untrusted'}
  '0x800B0101' {'certificate_expired'}
  '0x80073D02' {'package_in_use'}
  '0x80070005' {'access_denied'}
  default {if($script:setupStage -eq 'validate_package'){'package_validation_failed'}elseif($script:setupStage -eq 'dependency_check'){'package_query_failed'}elseif($script:setupStage -eq 'publisher_trust'){'certificate_registration_failed'}else{'widget_install'}}
 }
 Complete-Setup $code 1 $hresult $activityId
}
