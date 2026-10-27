# Build, release e deploy

## Release Windows

1. Atualize de forma consistente apps/desktop/package.json, package-lock.json, src-tauri/Cargo.toml, tauri.conf.json e Cargo.lock.
2. Execute revisão da fonte pública, build/testes de interface e cargo test --workspace --locked.
3. Dentro de apps/desktop, npm ci e npm run build. Na raiz, cargo build --release --locked -p previus-next-desktop --bin previus-next-desktop.
4. Copie o binário para um diretório local de release, com nome previdenciar.io-VERSAO-windows-x64.exe. Não o adicione ao Git.
5. Gere SHA256SUMS.txt e notas honestas de escopo, testes e limitações. Marque versões experimentais como prerelease.
6. Publique executável e checksum no GitHub Releases. Não anexe base, CNIS, PDFs pessoais, logs, dumps ou perfil de teste nativo.

A distribuição inclui um executável portátil e um instalador NSIS. O instalador é assinado apenas para o atualizador do próprio aplicativo; não há assinatura de código Windows. Build e download não certificam compatibilidade universal, cálculo normativo ou fluxo completo nativo.

## Automação

CI verifica frontend e Rust no Windows. O workflow de release é disparado por tags v* ou manualmente, compila com lockfile e cria uma prerelease. Se a release já existe, não substitui seus arquivos silenciosamente. Os logs do runner não devem incluir dados de usuários: apenas fixtures sintéticas são usadas.

## GitHub Pages

Apenas site/ é enviado ao Pages pelo workflow. O deploy fornece apresentação, documentação e links de download. Ele não hospeda o motor Tauri/Rust nem recebe arquivos. A URL padrão é a do GitHub Pages; o nome previdenciar.io não significa que um domínio foi comprado/configurado. Não há CNAME nesta entrega.

## Lista antes de publicar

- Repositório público é a cópia limpa, nunca a pasta de investigação privada.
- Nenhum dado pessoal, arquivo de base, segredo ou software proprietário está rastreado.
- Licença e notices de componentes/fontes estão presentes.
- Testes sintéticos, documentação de limites e issue do editor de contribuições estão disponíveis.
- Assets da marca definitiva só entram após aprovação e checagem de licença.

## Gate nativo obrigatório

Nunca execute o build nativo ao mesmo tempo em que Vite recria dist. Conclua npm run build, depois cargo build --release e scripts/smoke-native.ps1. O smoke abre um perfil separado e verifica os assets JS/CSS finais e a tela inicial real no WebView2; não certifica todos os fluxos nativos. O workflow recusa empacotamento se esse gate falhar.
