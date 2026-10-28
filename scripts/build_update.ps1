param([string]$ToolchainRoot='', [string]$OutputDirectory='')
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
if ($ToolchainRoot) {
 $toolchain=(Resolve-Path -LiteralPath $ToolchainRoot).Path
 $env:CARGO_HOME=Join-Path $toolchain 'cargo'; $env:RUSTUP_HOME=Join-Path $toolchain 'rustup'
 $env:PATH="$env:CARGO_HOME\bin;$env:PATH"; $env:RUSTUP_TOOLCHAIN='stable'
}
$config=Get-Content 'src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json
$version=$config.version
$frontendVersion=(Get-Content 'package.json' -Raw | ConvertFrom-Json).version
$cargoVersion=[regex]::Match((Get-Content 'src-tauri/Cargo.toml' -Raw),'(?m)^version = "([^"]+)"').Groups[1].Value
if ($version -ne $frontendVersion -or $version -ne $cargoVersion) {throw 'Synchronize config, frontend and Cargo versions before building'}
if (-not $OutputDirectory) {$OutputDirectory=Join-Path (Split-Path -Parent $projectRoot) "release-$version"}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$OutputDirectory=(Resolve-Path -LiteralPath $OutputDirectory).Path
$keyDirectory=Join-Path $env:LOCALAPPDATA 'KatsuTrackerReleaseKeys'
$keyPath=Join-Path $keyDirectory 'updater.key'
if (-not (Test-Path -LiteralPath $keyPath)) {throw 'Updater signing key is missing on this computer'}
$secure=Get-Content (Join-Path $keyDirectory 'password.dpapi') -Raw | ConvertTo-SecureString
$env:TAURI_SIGNING_PRIVATE_KEY=$keyPath
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD=[System.Net.NetworkCredential]::new('', $secure).Password
try {
 pnpm.cmd exec tauri build --bundles nsis -- --locked
 if ($LASTEXITCODE -ne 0) {throw 'Signed application build failed'}
} finally {
 Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
 Remove-Item Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
}
$installer=Get-ChildItem -LiteralPath 'src-tauri/target/release/bundle/nsis' -Filter '*-setup.exe' | Where-Object {$_.Name.Contains($version)} | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $installer) {throw 'Installer output was not found'}
$signaturePath=$installer.FullName+'.sig'
if (-not (Test-Path -LiteralPath $signaturePath)) {throw 'Updater signature is missing'}
Copy-Item -LiteralPath $installer.FullName -Destination (Join-Path $OutputDirectory 'Katsu-Tracker-setup.exe')
Copy-Item -LiteralPath $signaturePath -Destination (Join-Path $OutputDirectory 'Katsu-Tracker-setup.exe.sig')
Copy-Item -LiteralPath 'LICENSE' -Destination (Join-Path $OutputDirectory 'LICENSE.txt')
$manifest=@{
 version=$version; notes='Updater verification release: test the startup update notification, real download progress, silent background installation and automatic restart from 1.9.6-personal.36. Application features, visual Spike detection and saved preferences are unchanged.'
 pub_date=[DateTime]::UtcNow.ToString('o')
 platforms=@{'windows-x86_64'=@{
  signature=(Get-Content -LiteralPath $signaturePath -Raw).Trim()
  url="https://github.com/katsudc/katsu-tracker-releases/releases/download/v$version/Katsu-Tracker-setup.exe"
 }}
}
[System.IO.File]::WriteAllText((Join-Path $OutputDirectory 'latest.json'),($manifest | ConvertTo-Json -Depth 8),[System.Text.UTF8Encoding]::new($false))
$checksumLines=Get-ChildItem -LiteralPath $OutputDirectory -File | Where-Object {$_.Name -in @('Katsu-Tracker-setup.exe','Katsu-Tracker-setup.exe.sig','latest.json','LICENSE.txt')} | ForEach-Object {
 $hash=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
 "$hash  $($_.Name)"
}
$checksumLines | Set-Content -LiteralPath (Join-Path $OutputDirectory 'SHA256.txt') -Encoding utf8
Write-Output "Signed update ready: $OutputDirectory"
