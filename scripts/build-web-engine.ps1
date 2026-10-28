# Builds rgps-core for wasm32 and refreshes the browser assets under web/.
# Requires Rust + rustup. The generated engine.wasm and engine.data.js are
# reproducible from this repository; no network is used at runtime.
$ErrorActionPreference = 'Continue'
$repo = Split-Path -Parent $PSScriptRoot

$rustup = Start-Process -FilePath 'rustup' -ArgumentList @('target', 'add', 'wasm32-unknown-unknown') -Wait -PassThru -NoNewWindow
if ($rustup.ExitCode -ne 0) { throw 'rustup target add failed' }

cargo build --manifest-path (Join-Path $repo 'Cargo.toml') -p previus-wasm --target wasm32-unknown-unknown --release
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

node (Join-Path $PSScriptRoot 'embed-web-engine.js')
if ($LASTEXITCODE -ne 0) { throw 'asset embed failed' }

Write-Host 'Web engine assets refreshed: web/engine.wasm, web/engine.data.js, web/engine.example.*'
