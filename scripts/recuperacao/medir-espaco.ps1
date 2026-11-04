#requires -Version 5.1
<#
.SYNOPSIS
Mede o espaco do repo e do target em modo somente leitura (sem cargo, sem build, sem exclusao).

.DESCRIPTION
Le volume C:, pagefile, total do repo, target/ por subdiretorio, extensoes e worktrees Git.
NAO grava, NAO apaga, NAO move, NAO compila e NAO chama cargo.

.PARAMETER RepoRoot
Raiz do checkout. Padrao: dois niveis acima deste script.

.PARAMETER IncludeWorktrees
Quando presente, mede tambem o tamanho das worktrees Git listadas fora do repo.

.PARAMETER Json
Emite o resultado em JSON em vez de texto.

.EXAMPLE
pwsh -NoProfile -File scripts/recuperacao/medir-espaco.ps1

.EXAMPLE
pwsh -NoProfile -File scripts/recuperacao/medir-espaco.ps1 -Json
#>
param(
  [string]$RepoRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
  [switch]$IncludeWorktrees,
  [switch]$Json
)

$ErrorActionPreference = "Stop"

function Get-BytesText([long]$Bytes) {
  if ($Bytes -ge 1GB) { return ("{0:N2} GiB" -f ($Bytes / 1GB)) }
  if ($Bytes -ge 1MB) { return ("{0:N2} MiB" -f ($Bytes / 1MB)) }
  return ("{0:N0} B" -f $Bytes)
}

function Get-Tree([string]$Path) {
  if (-not (Test-Path -LiteralPath $Path)) { return $null }
  $bytes = [long]0; $files = 0
  Get-ChildItem -LiteralPath $Path -Recurse -Force -File -ErrorAction SilentlyContinue | ForEach-Object { $bytes += $_.Length; $files += 1 }
  [pscustomobject]@{ Path = $Path; Bytes = $bytes; Files = $files }
}

$root = (Resolve-Path -LiteralPath $RepoRoot).Path
$target = Join-Path $root "target"
$result = [ordered]@{ generatedAt = (Get-Date).ToString("o"); repoRoot = $root }

$drive = Get-PSDrive -Name C -ErrorAction SilentlyContinue
if ($drive) { $result.volumeC = [ordered]@{ usedBytes = [long]$drive.Used; freeBytes = [long]$drive.Free } }

$pf = Get-CimInstance Win32_PageFileUsage -ErrorAction SilentlyContinue | Select-Object -First 1
if ($pf) { $result.pagefileBytes = [long]$pf.AllocatedBaseSize * 1MB }

$result.repo = Get-Tree $root
$targetExists = Test-Path -LiteralPath $target
if ($targetExists) {
  $t = Get-Tree $target
  $result.target = $t
  $children = [ordered]@{}
  foreach ($name in @("debug", "release", "wasm32-unknown-unknown", "tmp")) { $children[$name] = Get-Tree (Join-Path $target $name) }
  $result.targetChildren = $children
  $debug = Join-Path $target "debug"
  if (Test-Path -LiteralPath $debug) {
    $dbg = [ordered]@{}
    foreach ($name in @("deps", "incremental", "examples", "build", ".fingerprint")) { $dbg[$name] = Get-Tree (Join-Path $debug $name) }
    $result.debugChildren = $dbg
  }
  $exts = @(".pdb", ".rlib", ".rmeta", ".o", ".exe")
  $acc = [ordered]@{}
  foreach ($e in $exts) { $acc[$e] = [pscustomobject]@{ Bytes = [long]0; Files = 0 } }
  Get-ChildItem -LiteralPath $target -Recurse -Force -File -ErrorAction SilentlyContinue | ForEach-Object {
    $e = $_.Extension.ToLowerInvariant()
    if ($acc.Contains($e)) { $acc[$e].Bytes += $_.Length; $acc[$e].Files += 1 }
  }
  $result.targetExtensions = $acc
}

$worktrees = @()
$git = Get-Command git -ErrorAction SilentlyContinue
if ($git) {
  try {
    $raw = & git -C $root worktree list --porcelain 2>$null
    if ($LASTEXITCODE -eq 0) { foreach ($line in $raw) { if ($line -like "worktree *") { $worktrees += $line.Substring(9) } } }
  } catch { $worktrees = @() }
}
$result.worktrees = @($worktrees)
if ($IncludeWorktrees) {
  $sizes = @()
  foreach ($w in $worktrees) { if (Test-Path -LiteralPath $w) { $sizes += Get-Tree $w } }
  $result.worktreeSizes = @($sizes)
}

$code = 0
if (-not $targetExists) { $code = 2 }
if ($Json) {
  $result | ConvertTo-Json -Depth 8
} else {
  $repo = $result.repo
  "== medir-espaco (somente leitura) =="
  "repo: $root"
  if ($repo) { "repo total: {0} ({1:N0} B, {2:N0} arquivos)" -f (Get-BytesText $repo.Bytes), $repo.Bytes, $repo.Files }
  if ($drive) { "volume C: usado {0} / livre {1}" -f (Get-BytesText $drive.Used), (Get-BytesText $drive.Free) }
  if ($result.pagefileBytes) { "pagefile: {0}" -f (Get-BytesText $result.pagefileBytes) }
  if ($targetExists) {
    "target: {0} ({1:N0} B, {2:N0} arquivos)" -f (Get-BytesText $t.Bytes), $t.Bytes, $t.Files
    if ($result.Contains("debugChildren")) {
      foreach ($k in $result.debugChildren.Keys) {
        $v = $result.debugChildren[$k]
        if ($v) { "  debug/{0}: {1} ({2:N0} arquivos)" -f $k, (Get-BytesText $v.Bytes), $v.Files }
      }
    }
    foreach ($k in $result.targetExtensions.Keys) {
      $v = $result.targetExtensions[$k]
      if ($v -and $v.Files -gt 0) { "  *{0}: {1} ({2:N0} arquivos)" -f $k, (Get-BytesText $v.Bytes), $v.Files }
    }
  } else {
    "target: ausente neste checkout; nada a medir (exit 2)"
  }
  "worktrees Git: {0}" -f $result.worktrees.Count
  foreach ($w in $result.worktrees) { "  $w" }
  if ($result.Contains("worktreeSizes")) {
    foreach ($s in $result.worktreeSizes) { "  worktree {0}: {1} ({2:N0} arquivos)" -f $s.Path, (Get-BytesText $s.Bytes), $s.Files }
  }
}
exit $code

