# Diferenças e plano de implementação

Data: 28/10/2026.

> Atualização: a sessão autenticada foi realizada. As observações de importação,
> PBC, otimização e diferenças de fluxo entre Pro e Calc estão em
> `2026-10-28-sessao-autenticada.md`. As linhas “não observado” abaixo continuam
> válidas apenas para o que não foi exercitado.

## O que foi observado

- O IEPREV Pro público exige login e não oferece Google/Microsoft na tela inicial observada.
- O IEPREV Calc sem sessão exibe erro de autenticação.
- Os bundles públicos mostram rotas de importação CNIS, RGPS, contagem de tempo, contribuições, benefícios, atrasos e transições; isso é indício de arquitetura, não prova de execução.
- O importador público do Calc usa PDF.js e concatena texto de páginas.
- O CNIS local autorizado foi lido pelo parser Rust após a correção de tipos de filiação; não foi publicada nenhuma identidade ou valor.

## O que não foi observado

- Cálculos autenticados do Pro ou do Calc.
- Parâmetros internos, memória completa, regras de seleção ou RMI.
- Como o Pro reutiliza o Calc.
- Relatórios autenticados e seus fundamentos.
- Diferenças reais entre os dois motores.

## Comparação de maturidade

| Capacidade | previdenciar.io local | IEPREV Calc | IEPREV Pro |
|---|---|---|---|
| CNIS textual local | sim, com origem e avisos | provável via PDF.js | provável via importador |
| CNIS imagem/OCR | server-side no webapp; não no parser Rust | não observado | não observado |
| RGPS espécie 42 | sim, experimental | não observado | não observado |
| Demais espécies | bloqueadas | não observado | não observado |
| Transições EC 103 | pontos, idade, pedágios | não observado | rota pública |
| PcD LC 142 | cenário experimental | não observado | não observado |
| Planejamento/liquidação | fora do motor | menus observados | rotas públicas |
| Relatórios PDF | renderer Rust | não observado | exportação oferecida |
| Persistência local cifrada | sim | não observado | backend remoto |
| Auditoria de origem | forte no parser local | não observado | não observado |

“Não observado” significa **desconhecido**, não ausência.

## Prioridades de implementação

1. **Importador CNIS**: manter a correção de tipos desconhecidos; tratar as 4 linhas não adjacentes restantes; testar com PDFs sintéticos de layouts distintos.
2. **Cobertura de regras**: criar catálogo por espécie e por regra; só liberar cálculo quando todos os parâmetros tiverem fonte e validação.
3. **Comparador**: executar a mesma matriz contra Calc, Pro e motor local; registrar diferenças como hipóteses.
4. **Relatórios**: ampliar o renderer Rust para incluir proveniência, regras rejeitadas, memória e limites já presentes no cálculo.
5. **Webapp**: manter o motor Rust como fonte; portar via WASM/API sem duplicar fórmulas e sem enviar CNIS a terceiros por padrão.
6. **Testes**: transformar cada cenário da matriz em fixture sintética e teste de regressão.

## Bloqueio atual

A sessão autenticada do IEPREV não está acessível ao segundo navegador: o perfil do `chrome-devtools-mcp` está em uso e os cookies estão bloqueados; os perfis Chrome normais não contêm sessão IEPREV. A conta não foi autenticada por credenciais extraídas. Enquanto isso, nenhuma conclusão autenticada de cálculo ou relatório será registrada.
