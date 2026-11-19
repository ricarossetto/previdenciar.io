# Inicia uma instancia local descartavel do servidor web do Previdenciar.
# NAO compila, NAO usa producao, NAO remove volumes. Exige binario obtido de build autorizado.
param(
  [Parameter(Mandatory=$true)][string]$BinaryPath,
  [string]$WebDist,
  [string]$ContainerName = 'previdenciar_test_a67ec8e86ef9',
  [int]$Port = 8788,
  [switch]$ConfirmDisposable,
  [string]$ExpectedSha256
)
$ErrorActionPreference = 'Stop'
if (-not $ConfirmDisposable) { throw 'Use -ConfirmDisposable. Este script inicia o container PG descartavel e um servidor local.' }
if (-not (Test-Path -LiteralPath $BinaryPath)) { throw "Binario nao encontrado: $BinaryPath" }
$bin = Get-Item -LiteralPath $BinaryPath
$hash = (Get-FileHash -LiteralPath $bin.FullName -Algorithm SHA256).Hash
if ($ExpectedSha256 -and $hash -ne $ExpectedSha256.ToUpperInvariant()) { throw "SHA-256 do binario nao confere. Esperado $ExpectedSha256, obtido $hash" }
if ($bin.LastWriteTime -lt (Get-Date).AddDays(-2)) { throw "Binario antigo (LastWriteTime=$($bin.LastWriteTime)). Use o artefato do build atual ou -AllowStaleBinary (nao implementado por seguranca)." }
if (-not $WebDist) { $WebDist = Join-Path $PSScriptRoot '..\..\apps\desktop\dist' }
$WebDist = (Resolve-Path -LiteralPath $WebDist).Path
$docker = Get-Command docker -ErrorAction SilentlyContinue
if (-not $docker) { throw 'Docker CLI ausente.' }
$state = (& docker inspect --format '{{.State.Status}}' $ContainerName 2>$null)
if ($LASTEXITCODE -ne 0) { throw "Container descartavel nao encontrado: $ContainerName. Nao crie outro sem revisar docs/recuperacao/AMBIENTE-LOCAL.md." }
if ($state -ne 'running') {
  & docker start $ContainerName | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'Falha ao iniciar o container descartavel.' }
  Start-Sleep -Seconds 3
}
$portLine = (& docker port $ContainerName 5432/tcp 2>$null | Select-Object -First 1)
if (-not $portLine -or $portLine -notmatch ':(\d+)$') { throw 'Nao foi possivel obter a porta PostgreSQL publicada no loopback.' }
$pgPort = [int]$Matches[1]
$envLines = @(& docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' $ContainerName)
function Get-EnvValue([string]$prefix) {
  $line = $envLines | Where-Object { $_ -like "$prefix=*" } | Select-Object -First 1
  if (-not $line) { throw "Variavel $prefix ausente no container." }
  return $line.Substring($prefix.Length + 1)
}
$user = Get-EnvValue 'POSTGRES_USER'; $db = Get-EnvValue 'POSTGRES_DB'; $password = Get-EnvValue 'POSTGRES_PASSWORD'
$encodedPassword = [System.Uri]::EscapeDataString($password)
$databaseUrl = "postgres://${user}:${encodedPassword}@127.0.0.1:${pgPort}/${db}"
$serverEnv = @{
  APP_ENV = 'development'
  AUTH_DISABLED = '1'
  DATABASE_URL = $databaseUrl
  BIND_ADDR = "127.0.0.1:$Port"
  WEB_DIST = $WebDist
  ALLOWED_ORIGINS = "http://127.0.0.1:$Port"
}
foreach($key in $serverEnv.Keys) { [Environment]::SetEnvironmentVariable($key, $serverEnv[$key], 'Process') }
Write-Output "Binario: $($bin.FullName) (SHA-256 $hash)"
Write-Output "Banco descartavel: $ContainerName em 127.0.0.1:$pgPort (credenciais nao exibidas)."
Write-Output "Iniciando servidor em http://127.0.0.1:$Port ..."
$proc = Start-Process -FilePath $bin.FullName -NoNewWindow -PassThru
$healthy = $false
for($i=0;$i -lt 40;$i++){
  Start-Sleep -Milliseconds 500
  if($proc.HasExited){ throw "Servidor encerrou com codigo $($proc.ExitCode)." }
  try { $health = Invoke-WebRequest "http://127.0.0.1:$Port/api/health" -UseBasicParsing -TimeoutSec 2; if($health.StatusCode -eq 200){ $healthy = $true; break } } catch { }
}
if(-not $healthy){ try { Stop-Process -Id $proc.Id -Force } catch { }; throw "Servidor iniciou, mas /api/health nao respondeu 200 em 20 s; processo encerrado." }
Write-Output "Servidor PID=$($proc.Id). Abra http://127.0.0.1:$Port/ e autentique manualmente se necessario."
