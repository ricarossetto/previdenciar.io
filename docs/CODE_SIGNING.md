# Assinatura de código, instalador e atualizações

## Instalador (já configurado)

`tauri.conf.json` já gera um instalador NSIS (`bundle.targets: ["nsis"]`). O
build produz:

- `previdenciar.io_<versao>_x64-setup.exe` — instalador por usuário (não pede administrador)
- `...setup.exe.sig` — assinatura do updater (minisign), gerada porque
  `createUpdaterArtifacts: true`

Para gerar localmente:

```powershell
cd apps/desktop
$env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.tauri\previdenciar.key"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
npx tauri build --bundles nsis
```

## Atualizações no app (já configurado)

- Plugin `tauri-plugin-updater` + `tauri-plugin-process` no Rust e no frontend.
- Endpoint: `https://github.com/ricarossetto/previdenciar.io/releases/latest/download/latest.json`
- Chave pública em `tauri.conf.json` (`plugins.updater.pubkey`).
- UI: **Configurações → Atualizações → Verificar atualizações**.

O workflow de release gera o `latest.json` automaticamente quando existe a
assinatura (`.sig`).

### Segredos obrigatórios no GitHub

Em **Settings → Secrets and variables → Actions**, crie:

| Segredo | Valor |
|---------|-------|
| `TAURI_SIGNING_PRIVATE_KEY` | conteúdo do arquivo `previdenciar.key` (texto base64) |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | senha da chave (vazio se gerada com `--ci`) |

A chave privada foi gerada em `%USERPROFILE%\.tauri\previdenciar.key`. **Guarde-a**:
se perdê-la, os updates param de funcionar (a pública já está fixada no app).

## Assinatura do executável (SmartScreen) — requer certificado

O updater usa minisign (próprio do Tauri). Isso **não** é o mesmo que assinatura
de código do Windows, que remove o aviso do SmartScreen. Para isso precisa de um
certificado de Code Signing. Opções, da mais prática à mais tradicional:

| Opção | Custo aprox. | Token físico | CI | Observação |
|-------|--------------|--------------|----|------------|
| **Azure Trusted Signing** | ~US$ 10/mês | não | sim | Melhor custo-benefício hoje; integra com GitHub Actions |
| Certificado OV | US$ 200–400/ano | sim (obrigatório desde 2023) | com token na máquina | Aviso do SmartScreen some com reputação gradual |
| Certificado EV | US$ 300–600/ano | sim | com token na máquina | Reputação imediata no SmartScreen |
| SignPath (OSS) | grátis para open source | não | sim | Requer o projeto ser open source |

### Como ligar no Tauri (depois de ter o certificado)

No `tauri.conf.json`, dentro de `bundle.windows`, use um dos caminhos:

```jsonc
// Azure Trusted Signing (via CLI dlib ou signtool)
"signCommand": { "cmd": "signtool", "args": ["sign", "/fd", "SHA256", "/tr", "http://timestamp.acs.microsoft.com", "/td", "SHA256", "/dlib", "<caminho>/Azure.CodeSigning.Dlib.dll", "/dmdf", "<caminho>/metadata.json", "%1"] }

// Certificado em arquivo .pfx
"certificateThumbprint": null,
"digestAlgorithm": "sha256",
"timestampUrl": "http://timestamp.digicert.com"
```

No CI, o certificado/token vai como segredo e o `signCommand` é chamado pelo
`tauri build`. O campo `BUILDINFO.json` hoje registra `"signed": false`; passa a
`true` quando a assinatura for ligada.

## Resumo do estado atual

- **Instalador:** sim (NSIS, por usuário)
- **Updater no app:** sim (falta só cadastrar os 2 segredos no GitHub)
- **Assinatura de código (SmartScreen):** não — depende de certificado a contratar
