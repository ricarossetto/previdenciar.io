# Motor local (WASM)

O `rgps-core` — o motor observacional de RGPS do repositório — compilado para `wasm32-unknown-unknown` e executado no navegador. É o núcleo de cálculo, não o app inteiro.

## O que roda e o que não roda

| Camada | No navegador? |
| --- | --- |
| `rgps-core` (`calcular`, `RgpsEntrada`, `RgpsOpcoes`) | Sim, via wasm |
| `previus-rgps` (adaptador, políticas, `calculate_json`) | Não; depende de CNIS/workspace no desktop |
| `previus-cnis` (importação de PDF) | Não |
| `previus-workspace` (base cifrada/DPAPI) | Não |
| `previus-reports` (PDF) | Não |
| Tauri / rede / filesystem | Não |

O wasm não tem WASI, filesystem, rede ou relógio de sistema. A entrada e a saída ficam na memória da página.

## Artefatos

| Arquivo | Tamanho | Papel |
| --- | --- | --- |
| `engine.wasm` | 453.628 bytes | build wasm32 do `rgps-core` |
| `engine.data.js` | 604.869 bytes | base64 do wasm para `file://` |
| `engine.example.js` / `.json` | 29 KB / 44 KB | fixture sintético de 300 competências |
| `engine.js` | ~2 KB | loader da ABI e da memória |
| `views-engine.js` | ~10 KB | página Motor local |

O wasm é embutido em base64 porque `fetch('engine.wasm')` é bloqueado em `file://` no Chrome. O `.wasm` também é servido ao lado para download e para uso em servidor.

## Rebuild

```powershell
scripts\build-web-engine.ps1
```

Ou manualmente:

```powershell
rustup target add wasm32-unknown-unknown
cargo build -p previus-wasm --target wasm32-unknown-unknown --release
node scripts\embed-web-engine.js
```

O script copia o wasm, gera o base64, copia o fixture e regenera `engine.example.js`. Nada é baixado em tempo de execução.

## ABI

Exportações em `crates/previus-wasm/src/lib.rs`:

| Export | Contrato |
| --- | --- |
| `pio_alloc(len) -> *mut u8` | aloca `len` bytes para a entrada |
| `pio_free(ptr, len)` | libera o buffer |
| `pio_calculate(ptr, len) -> *mut u8` | lê JSON UTF-8, calcula e devolve um buffer com prefixo `u64` little-endian e JSON |
| `pio_version() -> *mut u8` | devolve `{ engine, version, wasm, first_competence, divisor_minimo, reform }` |

O buffer de retorno começa com 8 bytes de comprimento (`u64` LE), seguidos do JSON. O chamador lê o comprimento, decodifica e chama `pio_free(ptr, 8 + len)`. A entrada é liberada depois do cálculo.

## Entrada

O JSON aceita o formato do adaptador e aliases em camelCase/snake_case:

```json
{
  "nome": "Helena Fagundes",
  "especie": "42",
  "segurado": {
    "sexo": "feminino",
    "dataNascimento": "1968-02-14",
    "deficiencia": "nao",
    "especie": "42"
  },
  "dataReferencia": "2026-08-01",
  "vinculos": [
    {
      "inicio": "1990-03-01",
      "fim": "1997-12-31",
      "tipo": "empregado",
      "empresa": "Metalúrgica Andrade S/A",
      "fator": 1
    }
  ],
  "competencias": [
    { "competencia": "2026-05", "salario": "4120.00", "confirmada": true }
  ],
  "opcoes": {
    "corrigirSalarios": false,
    "descarte": false,
    "divisorMinimo": false,
    "aplicarTetoVigenteDer": true
  }
}
```

Campos de `opcoes`: `corrigirSalarios`, `descarte`, `divisorMinimo`, `descartarMenoresMinimo`, `referenciaCorrecao`, `teto`, `piso`, `aplicarTetoVigenteDer`, `expectativaSobrevida`, `graca`, `recolhimentosAtraso`, `regraAlvo`. Datas em `AAAA-MM-DD`, competências em `AAAA-MM`, dinheiro em string decimal com ponto. O parser não aceita float binário nem separador brasileiro para evitar ambiguidade; a página converte valores do workspace antes de enviar.

## Saída

```json
{
  "ok": true,
  "engine": "rgps-core",
  "version": "0.1.0",
  "data": {
    "primeira_competencia": "2016-01",
    "ultima_competencia": "2025-12",
    "resumo": {
      "competencias": 300,
      "carencia_meses": 300,
      "salario_beneficio": "2000.00",
      "coeficiente": "80",
      "rmi": "1600.00"
    },
    "avisos": ["..."]
  }
}
```

Falha de schema/domínio devolve `{ "ok": false, "error": "..." }`. O erro é uma string legível; nenhum valor pessoal é ecoado.

## Página Motor local

Duas execuções:

- **Exemplo sintético**: `engine.example.js`, 300 competências, o mesmo fixture usado pelo teste nativo. Saída atual: 300 competências, coeficiente 80, RMI R$ 1.600,00.
- **Dados do caso**: monta vínculos a partir de `PERIODS` e competências a partir do livro de contribuições; no caso de demonstração são 12 competências e RMI R$ 1.728,50, com aviso de coeficiente de 60% por tempo/competências insuficientes. O resultado é do motor, não do valor cenográfico do caso.

A página mostra status (`Pronto`, `Calculado`, `Bloqueado`, `Erro`), o resumo em linhas de razão, os avisos e o JSON completo em `<details>`. O botão “Baixar wasm” entrega o binário; “Baixar exemplo” entrega o fixture.

## Verificação

- `cargo test -p previus-wasm`: 2 testes nativos (fixture 300 e entrada inválida).
- Node com `WebAssembly.instantiate`: versão, cálculo e leitura da memória.
- Chrome headless em `file://`: exemplo e caso executam, sem erro de console, sem requisição externa.
- `impeccable detect web`: 0 achados; `axe-core` WCAG 2 A/AA, 2.1 e 2.2 AA: 0 violações (incluindo a página do motor); 20 views a 390 px sem overflow.

## Limites

- É o `rgps-core`, não o adaptador `previus-rgps`; as políticas e bloqueios do adaptador (revisão confirmada, evidência, PcD, etc.) não são aplicados no wasm.
- O descarte do art. 26, § 6º, da EC 103/2019 está bloqueado no próprio núcleo; não há resultado financeiro até que o tempo e os requisitos sejam recalculados e validados.
- Não importa CNIS, não gera PDF, não lê a base cifrada e não substitui o aplicativo desktop.
- O resultado é experimental, não certifica direito e exige revisão profissional.
- O wasm é um artefato gerado; o build é reproduzível pelo script, mas o binário commitado deve ser regerado após mudanças no `rgps-core`.
