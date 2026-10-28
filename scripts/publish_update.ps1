param([Parameter(Mandatory=$true)][string]$OutputDirectory)
$ErrorActionPreference='Stop'
$OutputDirectory=(Resolve-Path -LiteralPath $OutputDirectory).Path
$manifest=Get-Content -LiteralPath (Join-Path $OutputDirectory 'latest.json') -Raw | ConvertFrom-Json
$version=$manifest.version
if ($version -notmatch '^\d+\.\d+\.\d+-personal\.\d+$') {throw 'Unexpected release version'}
$repo='katsudc/katsu-tracker-releases'
$expected="https://github.com/$repo/releases/download/v$version/Katsu-Tracker-setup.exe"
if ($manifest.platforms.'windows-x86_64'.url -ne $expected) {throw 'Unexpected update URL'}
$assets=@('Katsu-Tracker-setup.exe','Katsu-Tracker-setup.exe.sig','latest.json','LICENSE.txt','SHA256.txt')
foreach($name in $assets) {if(-not (Test-Path -LiteralPath (Join-Path $OutputDirectory $name))) {throw "Missing release file: $name"}}
$signature=(Get-Content -LiteralPath (Join-Path $OutputDirectory 'Katsu-Tracker-setup.exe.sig') -Raw).Trim()
if ($signature -ne $manifest.platforms.'windows-x86_64'.signature) {throw 'Manifest and signature file differ'}
foreach($line in (Get-Content -LiteralPath (Join-Path $OutputDirectory 'SHA256.txt'))) {
 if ($line -match '^([a-fA-F0-9]{64})  (.+)$') {
  $expectedHash=$Matches[1]; $name=$Matches[2]
  if ($name -notin $assets) {throw 'Unexpected checksum asset'}
  if ((Get-FileHash -LiteralPath (Join-Path $OutputDirectory $name) -Algorithm SHA256).Hash -ne $expectedHash) {throw "Checksum mismatch: $name"}
 }
}
$env:GIT_TERMINAL_PROMPT='0';$env:GCM_INTERACTIVE='never'
$lines="protocol=https`nhost=github.com`nusername=katsudc`n`n" | git credential fill 2>$null
$credentials=@{}
foreach($line in $lines) {$pair=$line -split '=',2;if($pair.Length -eq 2){$credentials[$pair[0]]=$pair[1]}}
if (-not $credentials.ContainsKey('password')) {throw 'Sign in to GitHub with Git Credential Manager first'}
$headers=@{Authorization="Bearer $($credentials['password'])";'User-Agent'='Katsu-Tracker-Releases';Accept='application/vnd.github+json'}
$user=Invoke-RestMethod 'https://api.github.com/user' -Headers $headers
if ($user.login -ne 'katsudc') {throw 'Unexpected publishing account'}
try {
 $latest=Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" -Headers $headers
 if ($latest.tag_name -notmatch '^v(\d+\.\d+\.\d+)-personal\.(\d+)$') {throw 'Latest version format is unexpected'}
 $latestCore=[Version]$Matches[1]; $latestSequence=[int]$Matches[2]
 $newParts=$version -split '-personal\.'
 if ([Version]$newParts[0] -lt $latestCore -or ([Version]$newParts[0] -eq $latestCore -and [int]$newParts[1] -le $latestSequence)) {throw 'Increment the application version before publishing'}
} catch {if ($_.Exception.Response -and [int]$_.Exception.Response.StatusCode -eq 404) {} else {throw}}
$readme=Get-Content -LiteralPath (Join-Path $PSScriptRoot '../docs/release-readme.md') -Raw -Encoding utf8
$currentReadme=Invoke-RestMethod "https://api.github.com/repos/$repo/contents/README.md" -Headers $headers
$readmeBody=@{message='Document installation and signed update channel';sha=$currentReadme.sha;content=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($readme))} | ConvertTo-Json
$null=Invoke-RestMethod "https://api.github.com/repos/$repo/contents/README.md" -Headers $headers -Method Put -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes($readmeBody))
$licenseText=Get-Content -LiteralPath (Join-Path $PSScriptRoot '../LICENSE') -Raw -Encoding utf8
$licenseBody=@{message='Update katsu project license';content=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($licenseText))}
try {
 $currentLicense=Invoke-RestMethod "https://api.github.com/repos/$repo/contents/LICENSE" -Headers $headers
 $licenseBody.sha=$currentLicense.sha
} catch {if ($_.Exception.Response -and [int]$_.Exception.Response.StatusCode -eq 404) {} else {throw}}
$null=Invoke-RestMethod "https://api.github.com/repos/$repo/contents/LICENSE" -Headers $headers -Method Put -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes(($licenseBody | ConvertTo-Json)))
$body=@{tag_name="v$version";target_commitish='main';name="katsu's Tracker $version";draft=$true;prerelease=$false;body='Visible startup update checks, explicit downloads with real progress, signature verification and a silent background installer with automatic restart. Customize the Spike countdown position, text size and opacity in a saved visual editor. Game-viewport/DPI placement and topmost handling improved; visual Spike detection is unchanged. Source code is now public.'} | ConvertTo-Json
$release=Invoke-RestMethod "https://api.github.com/repos/$repo/releases" -Headers $headers -Method Post -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes($body))
foreach($name in $assets) {
 $url="https://uploads.github.com/repos/$repo/releases/$($release.id)/assets?name=$([Uri]::EscapeDataString($name))"
 $null=Invoke-RestMethod $url -Headers $headers -Method Post -ContentType 'application/octet-stream' -InFile (Join-Path $OutputDirectory $name)
}
$published=Invoke-RestMethod "https://api.github.com/repos/$repo/releases/$($release.id)" -Headers $headers -Method Patch -ContentType 'application/json' -Body '{"draft":false,"make_latest":"true"}'
Write-Output $published.html_url
