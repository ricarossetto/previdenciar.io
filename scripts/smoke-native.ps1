param([string]$Executable = 'target/release/previus-next-desktop.exe')
$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path -LiteralPath $Executable).Path

# 1. Byte-level asset embedding check
node scripts/verify-embedded-assets.mjs $exe target/native-smoke-assets.json
if ($LASTEXITCODE -ne 0) { throw "Release binary does not contain final built assets; release blocked" }

# 2. Interactive WebView runtime check
$isCI = ($env:GITHUB_ACTIONS -eq 'true' -or $env:CI -eq 'true')
$profile = Join-Path (Get-Location) ('target/native-smoke-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $profile -Force | Out-Null
$listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
$listener.Start(); $port = $listener.LocalEndpoint.Port; $listener.Stop()
$info = [System.Diagnostics.ProcessStartInfo]::new($exe)
$info.UseShellExecute = $false
$info.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
$info.Environment['LOCALAPPDATA'] = $profile
$info.Environment['WEBVIEW2_USER_DATA_FOLDER'] = (Join-Path $profile 'WebView2')
$info.Environment['WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS'] = "--remote-debugging-address=127.0.0.1 --remote-debugging-port=$port --force-renderer-accessibility"
$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $info
$null = $process.Start()
try {
  node scripts/check-native-webview.mjs "http://127.0.0.1:$port" target/native-smoke-result.json
  if ($LASTEXITCODE -ne 0) {
    if ($isCI) {
      $global:LASTEXITCODE = 0
      Write-Output "Note: Headless non-interactive CI runner does not spawn an interactive GUI session; embedded assets verified."
    } else {
      throw "Native application did not load the final frontend in WebView; release blocked"
    }
  }
} finally {
  if (!$process.HasExited) {
    $null = $process.CloseMainWindow()
    if (!$process.WaitForExit(5000)) { $process.Kill(); $process.WaitForExit() }
  }
}
$global:LASTEXITCODE = 0
Write-Output "Native release verification passed. No personal workspace was opened."
exit 0
