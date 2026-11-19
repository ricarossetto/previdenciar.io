# Copies the public knowledge base audit extracts into the private per-user data
# directory used by the optional Copiloto. It never writes into the Git
# repository, never sends data over the network and never deletes existing
# files. PDFs are copied by default; use -SkipNatjusPdf to omit them. Text extraction from PDF happens in the app importer.
param(
    [string]$Source = '',
    [string]$Destination = (Join-Path $env:LOCALAPPDATA 'Previdenciar\conhecimento'),
    [switch]$SkipNatjusPdf
)
$copyNatjusPdf = -not $SkipNatjusPdf

$ErrorActionPreference = 'Stop'

function Require-Directory([string]$Path, [string]$Label) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "Diretorio de origem ausente ($Label): $Path"
    }
    (Resolve-Path -LiteralPath $Path).Path
}

function Copy-One([string]$From, [string]$To, [string]$Label) {
    if (-not (Test-Path -LiteralPath $From -PathType Leaf)) {
        throw "Arquivo de origem ausente ($Label): $From"
    }
    $parent = Split-Path -Parent $To
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    Copy-Item -LiteralPath $From -Destination $To -Force
}

function Copy-Tree([string]$From, [string]$To, [string]$Label) {
    if (-not (Test-Path -LiteralPath $From -PathType Container)) {
        throw "Pasta de origem ausente ($Label): $From"
    }
    if (-not (Test-Path -LiteralPath $To -PathType Container)) {
        New-Item -ItemType Directory -Force -Path $To | Out-Null
    }
    Copy-Item -Path (Join-Path $From '*') -Destination $To -Recurse -Force
}

function Find-PrefixedDirectory([string]$Root, [string]$Prefix, [string]$Label) {
    $matches = @(Get-ChildItem -LiteralPath $Root -Directory | Where-Object { $_.Name.ToUpperInvariant().StartsWith($Prefix) } | Sort-Object Name)
    if ($matches.Count -eq 0) {
        throw "Pasta de origem ausente ($Label): $Prefix*"
    }
    $matches[0].FullName
}
function Read-Json([string]$Path, [string]$Label) {
    try {
        Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
    } catch {
        throw "Falha ao ler JSON de $Label`: $Path"
    }
}

function Get-Text($Object, [string]$Name) {
    if ($null -eq $Object) { return $null }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property -or $null -eq $property.Value) { return $null }
    $value = [string]$property.Value
    if ([string]::IsNullOrWhiteSpace($value)) { return $null }
    $value.Trim()
}

if ([string]::IsNullOrWhiteSpace($Source)) { throw 'Informe -Source com a pasta da base de conhecimento.' }
if ([string]::IsNullOrWhiteSpace($Destination)) { throw 'Destino vazio recusado.' }
$Destination = [IO.Path]::GetFullPath($Destination)
if (-not [IO.Path]::IsPathRooted($Destination)) { throw 'O destino precisa ser absoluto.' }
$sourceRoot = Require-Directory $Source 'raiz da auditoria'
$destRoot = $Destination
if (-not (Test-Path -LiteralPath $destRoot -PathType Container)) {
    New-Item -ItemType Directory -Force -Path $destRoot | Out-Null
}

$skillSource = Find-PrefixedDirectory $sourceRoot 'HABILIDADES' 'habilidades'
$legislationSource = Find-PrefixedDirectory $sourceRoot 'LEGISLACAO' 'legislacao'
$natjusSource = Find-PrefixedDirectory $sourceRoot 'NATJUS' 'NatJus'
$jurisprudenceSource = Join-Path $sourceRoot 'JURISPRUDENCIA-API'

Copy-One (Join-Path $skillSource 'index.json') (Join-Path $destRoot 'skills\index.json') 'indice de habilidades'
Copy-Tree (Join-Path $skillSource 'md') (Join-Path $destRoot 'skills\md') 'textos de habilidades'
Copy-One (Join-Path $legislationSource 'index.json') (Join-Path $destRoot 'legislation\index.json') 'indice de legislacao'
Copy-Tree (Join-Path $legislationSource 'normas') (Join-Path $destRoot 'legislation\normas') 'normas em Markdown'
Copy-One (Join-Path $natjusSource 'revisoes.json') (Join-Path $destRoot 'natjus\revisoes.json') 'indice NatJus'
Copy-Tree (Join-Path $natjusSource 'revisoes') (Join-Path $destRoot 'natjus\revisoes') 'fichas NatJus'
if ($copyNatjusPdf) {
    Copy-Tree (Join-Path $natjusSource 'pdf') (Join-Path $destRoot 'natjus\pdf') 'PDFs NatJus'
}

$records = New-Object System.Collections.Generic.List[object]
$seen = @{}
$buckets = @('results', 'reranked_results', 'juris', 'rg', 'sumulas', 'sumulas_vinc', 'repetitivos', 'puil', 'iacs')
$files = @()
if (Test-Path -LiteralPath $jurisprudenceSource -PathType Container) {
    $files = @(Get-ChildItem -LiteralPath $jurisprudenceSource -Filter 'search-*.json' -File | Sort-Object Name)
}
foreach ($file in $files) {
    $tribunal = ($file.BaseName -replace '^search-', '').ToLowerInvariant()
    $document = Read-Json $file.FullName 'jurisprudencia'
    foreach ($bucket in $buckets) {
        $property = $document.PSObject.Properties[$bucket]
        if ($null -eq $property -or $null -eq $property.Value) { continue }
        foreach ($item in @($property.Value)) {
            $ementa = Get-Text $item 'texto_ementa'
            if (-not $ementa) { $ementa = Get-Text $item 'ementa' }
            if (-not $ementa) { $ementa = Get-Text $item 'texto' }
            if (-not $ementa) { continue }
            $id = Get-Text $item 'id'
            if (-not $id) { $id = Get-Text $item 'normalizedId' }
            if (-not $id) { $id = "sem-id-$($records.Count + 1)" }
            $key = "$tribunal-$id"
            $fingerprint = "$key|" + $ementa.Substring(0, [Math]::Min(160, $ementa.Length))
            if ($seen.ContainsKey($fingerprint)) { continue }
            $seen[$fingerprint] = $true
            $classe = Get-Text $item 'classe_processual'
            $numero = Get-Text $item 'numero_processo'
            $titulo = Get-Text $item 'titulo'
            if (-not $titulo) { $titulo = (@($classe, $numero) | Where-Object { $_ }) -join ' ' }
            if (-not $titulo) { $titulo = "$tribunal $id" }
            $url = Get-Text $item 'link_pdf'
            if (-not $url) { $url = Get-Text $item 'link_consulta' }
            if (-not $url) { $url = Get-Text $item 'link_processo' }
            if (-not $url) { $url = Get-Text $item 'sourceUrl' }
            $records.Add([ordered]@{
                key = $key
                id = $id
                tribunal = $tribunal
                tipo = (Get-Text $item '__kind')
                titulo = $titulo
                classe = $classe
                processo = (Get-Text $item 'numero_processo_cnj')
                relator = (Get-Text $item 'relator')
                orgao = (Get-Text $item 'orgao_julgador')
                data = (Get-Text $item 'data_julgamento')
                url = $url
                ementa = $ementa
                bucket = $bucket
            }) | Out-Null
        }
    }
}
if ($records.Count -eq 0) {
    Write-Warning 'Nenhuma ementa foi normalizada; o arquivo jurisprudence.json ficara vazio.'
}
$jurisprudenceJson = ConvertTo-Json -InputObject $records.ToArray() -Depth 10
[IO.File]::WriteAllText((Join-Path $destRoot 'jurisprudence.json'), $jurisprudenceJson, (New-Object System.Text.UTF8Encoding($false)))

$skills = Read-Json (Join-Path $destRoot 'skills\index.json') 'habilidades'
$legislation = Read-Json (Join-Path $destRoot 'legislation\index.json') 'legislacao'
$natjus = Read-Json (Join-Path $destRoot 'natjus\revisoes.json') 'NatJus'
$manifest = [ordered]@{
    importedAt = (Get-Date).ToUniversalTime().ToString('o')
    source = $sourceRoot
    destination = $destRoot
    skills = $skills.Count
    legislation = $legislation.Count
    natjus = $natjus.Count
    jurisprudence = $records.Count
    natjusPdfIncluded = [bool]$copyNatjusPdf
    natjusText = 0
    natjusTextNote = 'A extracao de texto dos PDFs e feita pelo importador do aplicativo.'
    note = 'Extratos publicos da base de conhecimento; a base local nao certifica vigencia, inteiro teor ou aplicacao ao caso.'
}
[IO.File]::WriteAllText((Join-Path $destRoot 'manifest.json'), (ConvertTo-Json -InputObject $manifest -Depth 5), (New-Object System.Text.UTF8Encoding($false)))

Write-Host ("Base local atualizada em {0}" -f $destRoot)
Write-Host ("Habilidades: {0} | Legislacao: {1} | NatJus: {2} | Jurisprudencia: {3}" -f $skills.Count, $legislation.Count, $natjus.Count, $records.Count)
if ($SkipNatjusPdf) { Write-Host 'PDFs NatJus nao foram copiados; use o importador do aplicativo para texto pesquisavel.' }
Write-Host 'Nenhum dado foi enviado pela rede.'




