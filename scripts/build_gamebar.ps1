param([switch]$SignWithLocalTestCertificate)
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
$buildRoot=Join-Path $projectRoot 'gamebar/build'
$packageRoot=Join-Path $buildRoot 'package'
New-Item -ItemType Directory -Path $packageRoot -Force | Out-Null
$sdkVersion='7.3.2607010'
$sdkHash='20230e0a1544ebb391764e13cdbaa1eda86239968444b599fcc9fa1e3d748ff1'
$cacheRoot=Join-Path $env:LOCALAPPDATA "KatsuTrackerBuildCache/gamebar-$sdkVersion"
$archivePath=Join-Path $cacheRoot 'sdk.zip'
New-Item -ItemType Directory -Path $cacheRoot -Force | Out-Null
if(-not (Test-Path -LiteralPath $archivePath)){
 Invoke-WebRequest "https://api.nuget.org/v3-flatcontainer/microsoft.gaming.xboxgamebar/$sdkVersion/microsoft.gaming.xboxgamebar.$sdkVersion.nupkg" -OutFile $archivePath -UseBasicParsing
}
if((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $sdkHash){throw 'Game Bar SDK integrity check failed'}
$sdkRoot=Join-Path $cacheRoot 'sdk'
if(-not (Test-Path -LiteralPath (Join-Path $sdkRoot 'license.txt'))){Expand-Archive -LiteralPath $archivePath -DestinationPath $sdkRoot -Force}
$vswhere=Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vsRoot=(& $vswhere -latest -version '[17.0,18.0)' -products '*' -property installationPath).Trim()
if(-not $vsRoot){throw 'Visual Studio 2022 C++ tools are required to build the widget'}
$vcVersion=(Get-Content (Join-Path $vsRoot 'VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt') -Raw).Trim()
$vcRoot=Join-Path $vsRoot "VC/Tools/MSVC/$vcVersion"
$windowsKit=Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10'
$kitVersion=(Get-ChildItem -LiteralPath (Join-Path $windowsKit 'UnionMetadata') -Directory | Where-Object {$_.Name -match '^10\.0\.\d+\.0$'} | Sort-Object {[version]$_.Name} -Descending | Select-Object -First 1).Name
if(-not $kitVersion){throw 'Windows 10 SDK is required'}
$kitTools=Join-Path $windowsKit "bin/$kitVersion/x64"
$source=Join-Path $projectRoot 'gamebar/widget/Widget.cpp'
$object=Join-Path $buildRoot 'Widget.obj'
$executable=Join-Path $packageRoot 'KatsuSpikeWidget.exe'
$devcmd=Join-Path $vsRoot 'Common7/Tools/VsDevCmd.bat'
$platform=Join-Path $vcRoot 'lib/x86/store/references/platform.winmd'
$metadata=Join-Path $windowsKit "UnionMetadata/$kitVersion/Windows.winmd"
$gamebar=Join-Path $sdkRoot 'lib/uap10.0/Microsoft.Gaming.XboxGameBar.winmd'
$storeLib=Join-Path $vcRoot 'lib/x64/store'
$compile=@"
@call "$devcmd" -arch=x64 -host_arch=x64 >nul
@cl /nologo /utf-8 /ZW /MD /EHsc /O2 /wd4691 /DWINAPI_FAMILY=WINAPI_FAMILY_APP /DUNICODE /D_UNICODE /FU"$platform" /FU"$metadata" /FU"$gamebar" /Fo"$object" /Fe"$executable" "$source" /link /APPCONTAINER /SUBSYSTEM:WINDOWS windowsapp.lib runtimeobject.lib /LIBPATH:"$storeLib"
"@
$command=Join-Path $buildRoot 'compile.cmd'
[IO.File]::WriteAllText($command,$compile,[Text.Encoding]::ASCII)
& $env:ComSpec /d /c $command
if($LASTEXITCODE -ne 0){throw 'Widget compilation failed'}
Copy-Item -LiteralPath (Join-Path $projectRoot 'gamebar/widget/Package.appxmanifest') -Destination (Join-Path $packageRoot 'AppxManifest.xml')
$config=Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
$parts=$config.version -split '-personal\.'
$manifest=[xml](Get-Content -LiteralPath (Join-Path $packageRoot 'AppxManifest.xml') -Raw)
$manifest.Package.Identity.Version="$($parts[0]).$($parts[1])"
$manifest.Save((Join-Path $packageRoot 'AppxManifest.xml'))
New-Item -ItemType Directory -Path (Join-Path $packageRoot 'Assets') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot 'src-tauri/icons/128x128@2x.png') -Destination (Join-Path $packageRoot 'Assets/Logo.png')
foreach($name in @('Microsoft.Gaming.XboxGameBar.dll','Microsoft.Gaming.XboxGameBar.pri')){Copy-Item -LiteralPath (Join-Path $sdkRoot "runtimes/win10-x64/native/$name") -Destination (Join-Path $packageRoot $name)}
Copy-Item -LiteralPath $gamebar -Destination (Join-Path $packageRoot 'Microsoft.Gaming.XboxGameBar.winmd')
Copy-Item -LiteralPath (Join-Path $sdkRoot 'private/Microsoft.Gaming.XboxGameBar.Private.winmd') -Destination (Join-Path $packageRoot 'Microsoft.Gaming.XboxGameBar.Private.winmd')
Copy-Item -LiteralPath (Join-Path $sdkRoot 'license.txt') -Destination (Join-Path $packageRoot 'Microsoft-GameBar-SDK-LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $projectRoot 'gamebar/THIRD_PARTY_NOTICES.md') -Destination (Join-Path $packageRoot 'THIRD_PARTY_NOTICES.md')
New-Item -ItemType Directory -Path (Join-Path $packageRoot 'GameBar') -Force | Out-Null
Set-Content -LiteralPath (Join-Path $packageRoot 'GameBar/Widget.txt') -Value 'katsu Spike Timer' -Encoding ascii
$package=Join-Path $buildRoot 'KatsuSpikeWidget.msix'
& (Join-Path $kitTools 'makeappx.exe') pack /d $packageRoot /p $package /o
if($LASTEXITCODE -ne 0){throw 'Widget packaging failed'}
if($SignWithLocalTestCertificate){
 $keyRoot=Join-Path $env:LOCALAPPDATA 'KatsuTrackerReleaseKeys'
 New-Item -ItemType Directory -Path $keyRoot -Force | Out-Null
 $thumbPath=Join-Path $keyRoot 'gamebar-test-certificate-thumbprint.txt'
 $certificate=$null
 if(Test-Path -LiteralPath $thumbPath){$thumb=(Get-Content -LiteralPath $thumbPath -Raw).Trim();$certificate=Get-Item -LiteralPath "Cert:\CurrentUser\My\$thumb" -ErrorAction SilentlyContinue}
 if(-not $certificate){
  $certificate=New-SelfSignedCertificate -Type CodeSigningCert -Subject 'CN=katsu' -FriendlyName 'katsu Tracker Game Bar local testing' -CertStoreLocation 'Cert:\CurrentUser\My' -KeyExportPolicy NonExportable -NotAfter (Get-Date).AddYears(1)
  Set-Content -LiteralPath $thumbPath -Value $certificate.Thumbprint -Encoding ascii
 }
 & (Join-Path $kitTools 'signtool.exe') sign /fd SHA256 /s My /sha1 $certificate.Thumbprint $package
 if($LASTEXITCODE -ne 0){throw 'Widget test signing failed'}
 $null=Export-Certificate -Cert $certificate -FilePath (Join-Path $buildRoot 'WidgetPublisher.cer') -Force
 Set-Content -LiteralPath (Join-Path $buildRoot 'WidgetPublisher.thumbprint') -Value $certificate.Thumbprint -Encoding ascii
 Write-Output 'Signed for LOCAL TESTING. Publisher trust is a separate, explicit first-install step.'
}
& (Join-Path $PSScriptRoot 'prepare_widget_runtime.ps1')
Write-Output "Game Bar package ready: $package"
