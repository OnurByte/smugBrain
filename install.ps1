# smugBrain installer for Windows: downloads the prebuilt smug.exe from GitHub Releases, verifies SHA-256, installs it.
# No Rust, no compiler. Never touches your notes.
#   $env:SMUG_VERSION = 'v0.2.0'   pick a release (default: latest)
$ErrorActionPreference = 'Stop'
$repo = 'OnurByte/smugbrain'
$asset = 'smug-x86_64-pc-windows-msvc.zip'
$base = if ($env:SMUG_BASE) { $env:SMUG_BASE } elseif ($env:SMUG_VERSION) { "https://github.com/$repo/releases/download/$($env:SMUG_VERSION)" } else { "https://github.com/$repo/releases/latest/download" }
$dest = Join-Path $env:LOCALAPPDATA 'smug\bin'
$tmp = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile "$tmp\$asset"
  Invoke-WebRequest -UseBasicParsing -Uri "$base/SHA256SUMS" -OutFile "$tmp\SHA256SUMS"
  $line = (Get-Content "$tmp\SHA256SUMS") | Where-Object { $_ -match [regex]::Escape($asset) + '$' } | Select-Object -First 1
  $want = if ($line) { ($line -split '\s+')[0] } else { '' }
  $got = (Get-FileHash "$tmp\$asset" -Algorithm SHA256).Hash.ToLower()
  if (-not $want -or $want.ToLower() -ne $got) { throw 'checksum mismatch, refusing to install' }
  Expand-Archive -Path "$tmp\$asset" -DestinationPath $tmp -Force
  New-Item -ItemType Directory -Path $dest -Force | Out-Null
  Copy-Item "$tmp\smug.exe" (Join-Path $dest 'smug.exe') -Force
  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  if (($userPath -split ';') -notcontains $dest) {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$dest", 'User')
    Write-Host "smug: added $dest to your user PATH (open a new terminal)"
  }
  Write-Host "smug: installed to $dest\smug.exe"
  Write-Host 'smug: next -> smug init my-brain'
} finally { Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue }
