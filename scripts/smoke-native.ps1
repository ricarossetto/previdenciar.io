param([string]$Executable = 'target/release/previus-next-desktop.exe')
$ErrorActionPreference='Stop'
$exe=(Resolve-Path -LiteralPath $Executable).Path
$profile=Join-Path (Get-Location) ('target/native-smoke-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $profile -Force|Out-Null
$listener=[System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback,0)
$listener.Start();$port=$listener.LocalEndpoint.Port;$listener.Stop()
$info=[System.Diagnostics.ProcessStartInfo]::new($exe)
$info.UseShellExecute=$false
$info.WindowStyle=[System.Diagnostics.ProcessWindowStyle]::Hidden
$info.Environment['LOCALAPPDATA']=$profile
$info.Environment['WEBVIEW2_USER_DATA_FOLDER']=(Join-Path $profile 'WebView2')
$info.Environment['WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS']="--remote-debugging-address=127.0.0.1 --remote-debugging-port=$port --force-renderer-accessibility"
$process=[System.Diagnostics.Process]::new();$process.StartInfo=$info
$null=$process.Start()
try{
  node scripts/check-native-webview.mjs "http://127.0.0.1:$port" target/native-smoke-result.json
  if($LASTEXITCODE -ne 0){throw 'Native application did not load the final frontend; release blocked'}
}finally{
  if(!$process.HasExited){$null=$process.CloseMainWindow();if(!$process.WaitForExit(5000)){$process.Kill();$process.WaitForExit()}}
}
Write-Output 'Native WebView smoke passed in a separate profile. No personal workspace was opened.'
