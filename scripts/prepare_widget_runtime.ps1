# Extract the unmodified Microsoft-signed retail UWP framework. Does not install anything.
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
$cabHash='dd4dbe036d235dd902ef41f2cb3ec7ce7ab1ebf93e6cc26fb6d7283301188e5a'
$appxHash='9c17b521f9d690a1f504da5108ed6eec5669eb3a8fd1331eef43e40d84e74283'
$cache=Join-Path $env:LOCALAPPDATA 'KatsuTrackerBuildCache/vclibs-14.44.35208'
New-Item -ItemType Directory -Path $cache -Force | Out-Null
$cab=Join-Path $cache 'cab1.cab'
if(-not(Test-Path -LiteralPath $cab)){
 Invoke-WebRequest -Uri "https://download.visualstudio.microsoft.com/download/pr/d2fa077f-a814-4fb2-b903-1fca7658d17e/$cabHash/cab1.cab" -OutFile $cab -UseBasicParsing
}
if((Get-FileHash -LiteralPath $cab).Hash.ToLowerInvariant() -ne $cabHash){throw 'Microsoft UWP framework archive hash mismatch'}
$output=Join-Path $projectRoot 'gamebar/build/Dependencies'
New-Item -ItemType Directory -Path $output -Force | Out-Null
& (Join-Path $env:SystemRoot 'System32/expand.exe') '-F:Microsoft.VCLibs.x64.14.00.appx' $cab $output | Out-Null
if($LASTEXITCODE -ne 0){throw 'Microsoft UWP framework extraction failed'}
$framework=Join-Path $output 'Microsoft.VCLibs.x64.14.00.appx'
if((Get-FileHash -LiteralPath $framework).Hash.ToLowerInvariant() -ne $appxHash){throw 'Microsoft UWP framework package hash mismatch'}
Write-Output 'Microsoft.VCLibs.140.00 x64 retail framework prepared; no software installed.'
