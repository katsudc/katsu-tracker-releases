param([string]$ToolchainRoot='')
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
if($ToolchainRoot){
 $toolchain=(Resolve-Path -LiteralPath $ToolchainRoot).Path
 $env:CARGO_HOME=Join-Path $toolchain 'cargo';$env:RUSTUP_HOME=Join-Path $toolchain 'rustup'
 $env:RUSTUP_TOOLCHAIN='stable';$env:PATH="$env:CARGO_HOME\bin;$env:PATH"
}
& (Join-Path $PSScriptRoot 'build_gamebar.ps1') -SignWithLocalTestCertificate
if($LASTEXITCODE -ne 0){throw 'Game Bar build failed'}
# Invoke the CLI directly so package-manager argument parsing preserves Cargo's --.
node (Join-Path $projectRoot 'node_modules/@tauri-apps/cli/tauri.js') build --no-bundle -- --locked
if($LASTEXITCODE -ne 0){throw 'Local tracker build failed'}
$resources=Join-Path $projectRoot 'src-tauri/target/release/gamebar'
New-Item -ItemType Directory -Path $resources -Force | Out-Null
foreach($name in @('KatsuSpikeWidget.msix','WidgetPublisher.cer')){Copy-Item -LiteralPath (Join-Path $projectRoot "gamebar/build/$name") -Destination (Join-Path $resources $name)}
Copy-Item -LiteralPath (Join-Path $projectRoot 'gamebar/install_widget.ps1') -Destination (Join-Path $resources 'install_widget.ps1')
Write-Output "LOCAL ONLY: $(Join-Path $projectRoot 'src-tauri/target/release/valorant-tracker.exe')"
Write-Output 'No source, tag, installer or release has been published.'
