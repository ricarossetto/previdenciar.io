# DESIGN.md

## Direção: mesa de trabalho

A entrevista do assistente deixa de parecer um painel e passa a parecer um documento bem impresso sobre a mesa: fundo de papel, tinta escura, uma única cor de ação e muito espaço. O contraste é alto, o texto é grande e cada tela faz uma pergunta.

Escopo: telas do assistente (`flow-v2`), conferência e cabeçalho do resultado. O restante do aplicativo mantém a casca atual.

## Princípios

1. Uma pergunta por tela, com uma saída principal.
2. Palavra comum antes de sigla. DIB, RMI e PBC aparecem com explicação na primeira vez.
3. Nada de cartão dentro de cartão. Seções com divisória bastam.
4. O que o programa não sabe fica visível, em texto curto e sem drama.
5. O próximo passo está sempre no mesmo lugar: abaixo do conteúdo, alinhado à esquerda.

## Tokens (`flow-v2`)

| Token | Valor | Uso |
| --- | --- | --- |
| `--paper` | `#F7F4EC` | fundo da área de trabalho |
| `--paper-2` | `#FFFDF8` | campos e superfícies elevadas |
| `--ink` | `#1C1A17` | texto principal |
| `--ink-2` | `#4A453D` | texto secundário |
| `--rule` | `#D9D2C4` | divisórias e bordas |
| `--action` | `#1E4B39` | ação primária |
| `--action-ink` | `#F7F4EC` | texto sobre a ação |
| `--alert` | `#7A3B12` | bloqueio e pendência |
| `--ok` | `#1E4B39` | hipótese revisada |

Tipografia: Lora para títulos, Inter para corpo. Corpo 16 px, pergunta 32 px, opção 18 px, apoio 14 px. Medida de leitura até 68 ch.

## Componentes

- **Trilho de progresso**: "Passo 2 de 7" em texto simples, barra fina de 4 px, sem caixa alta e sem ícone.
- **Pergunta**: título serifado, uma linha de contexto, nada mais.
- **Opção**: linha inteira clicável, título 18 px, explicação 14 px, borda de 1 px; selecionada ganha borda de 2 px na tinta e um check desenhado.
- **Ação primária**: botão sólido, 48 px de altura, largura do conteúdo no mobile.
- **Aviso honesto**: texto com divisória à esquerda de 2 px em `--alert`, sem ícone e sem caixa alta.
- **Status do resultado**: faixa com o estado ("Cálculo bloqueado"), o motivo em uma frase e a próxima ação.

## Movimento

Uma entrada por tela: 8 px de deslocamento e opacidade, 220 ms, ease-out exponencial. Sem animação quando `prefers-reduced-motion: reduce`.

## Acessibilidade

Foco visível de 2 px em todas as ações, alvo mínimo de 44 px, contraste mínimo 4.5:1 no corpo, `aria-current` no passo atual, `role="status"` nos avisos que mudam.
