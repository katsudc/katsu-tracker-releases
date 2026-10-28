# Build from the reviewed source; never downloads the publisher's application.
param([string]$ToolchainRoot = '')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot

if ($ToolchainRoot) {
    $resolvedToolchain = (Resolve-Path -LiteralPath $ToolchainRoot).Path
    $env:CARGO_HOME = Join-Path $resolvedToolchain 'cargo'
    $env:RUSTUP_HOME = Join-Path $resolvedToolchain 'rustup'
    $env:PATH = "$env:CARGO_HOME\bin;$env:PATH"
}

pnpm.cmd install --frozen-lockfile --ignore-scripts
if ($LASTEXITCODE -ne 0) { throw 'Dependency installation failed' }

pnpm.cmd exec tauri build --no-bundle -- --locked
if ($LASTEXITCODE -ne 0) { throw 'Application build failed' }

cargo test --release --locked --manifest-path src-tauri/Cargo.toml --lib
if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }

$releaseDirectory = Join-Path $projectRoot 'release_output'
New-Item -ItemType Directory -Force -Path $releaseDirectory | Out-Null
$destination = Join-Path $releaseDirectory 'VALORANT-Tracker-Personal.exe'
Copy-Item -LiteralPath 'src-tauri/target/release/valorant-tracker.exe' -Destination $destination
Copy-Item -LiteralPath 'LICENSE' -Destination $releaseDirectory
Get-FileHash -Algorithm SHA256 -LiteralPath $destination
