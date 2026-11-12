# smugBrain installer for Windows PowerShell: copies the empty vault into .\smugbrain. Never overwrites.
$ErrorActionPreference = 'Stop'
$dest = 'smugbrain'
$url = if ($env:SMUG_ZIP) { $env:SMUG_ZIP } else { 'https://github.com/OnurByte/smugbrain/archive/refs/heads/main.zip' }
if (Test-Path $dest) { Write-Error "smug: '$dest' already exists, refusing to overwrite"; exit 1 }
$tmp = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile "$tmp\s.zip"
  Expand-Archive -Path "$tmp\s.zip" -DestinationPath "$tmp\x"
  $root = (Get-ChildItem "$tmp\x" -Directory | Select-Object -First 1).FullName
  Copy-Item -Path "$root\template" -Destination $dest -Recurse -Force
  Write-Host "smug: your brain has been replaced. Open '$dest' in Obsidian."
} finally { Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue }
