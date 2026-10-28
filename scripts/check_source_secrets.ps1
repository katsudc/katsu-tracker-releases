param([string]$SourceRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
$SourceRoot = (Resolve-Path -LiteralPath $SourceRoot).Path
$paths = @(git -C $SourceRoot -c core.quotepath=false ls-files --cached --others --exclude-standard) | Sort-Object -Unique
if ($LASTEXITCODE -ne 0) { throw 'Unable to list source files' }

# These exact public Google client identifiers were verified against Google's
# own widget/extension distribution. Never allow arbitrary Google API keys.
$publicTranslateHashes = @(
 '807d71110d3f08944575f529b82646799d37c820263bb2bcac019375d1e9659d',
 '83f395c814a1d173ddd06db410984fe2dc36f4f393c5e6eddd6db93f69cc1182'
)
$patterns = [ordered]@{
 GoogleApiKey = 'AIza[0-9A-Za-z_-]{35}'
 GitHubToken = '\b(gh[pousr]_[A-Za-z0-9]{25,}|github_pat_[A-Za-z0-9_]{30,})\b'
 OpenAiKey = '\bsk-(proj-|svcacct-)?[A-Za-z0-9_-]{30,}'
 AwsAccessKey = '\b(AKIA|ASIA)[A-Z0-9]{16}\b'
 PrivateKey = '-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----'
 JwtCredential = '\beyJ[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{15,}'
 CredentialUrl = 'https?://[^/\s:@]+:[^/\s@]+@'
}
$findings = [System.Collections.Generic.List[string]]::new()
$allowed = 0
$sha = [System.Security.Cryptography.SHA256]::Create()
try {
 foreach ($relative in $paths) {
  $path = Join-Path $SourceRoot $relative
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { continue }
  $normalized = $relative.Replace('\','/')
  $name = [IO.Path]::GetFileName($path)
  if ($name -match '^\.env($|\.)' -or $name -match '\.(key|dpapi|pfx|pem)$' -or
      ($name -match '\.p12$' -and $normalized -ne 'src-tauri/tests/fixtures/localhost-test-only.p12')) {
   $findings.Add("$normalized : private credential file"); continue
  }
  $file = Get-Item -LiteralPath $path
  if ($file.Length -gt 2MB -or $file.Extension -match '^\.(png|jpg|jpeg|ico|icns|p12|bgra|pcm16|woff|woff2)$') { continue }
  $contents = [IO.File]::ReadAllText($path)
  foreach ($kind in $patterns.Keys) {
   foreach ($match in [regex]::Matches($contents, $patterns[$kind])) {
    if ($kind -eq 'GoogleApiKey' -and $normalized -eq 'src-tauri/src/translate.rs') {
     $hash = [BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($match.Value))).Replace('-','').ToLowerInvariant()
     if ($hash -in $publicTranslateHashes) { $allowed++; continue }
    }
    $line = 1 + ([regex]::Matches($contents.Substring(0,$match.Index), "`n")).Count
    # Never print the credential value, even when reporting a failure.
    $findings.Add("${normalized}:$line : $kind")
   }
  }
 }
} finally { $sha.Dispose() }
if ($findings.Count) {
 foreach ($finding in $findings) { Write-Output $finding }
 throw 'Possible private credentials detected. Review before publishing.'
}
Write-Output "Source credential check passed. Verified public Translate client identifiers: $allowed."
