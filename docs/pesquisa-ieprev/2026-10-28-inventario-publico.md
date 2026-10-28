# IEPREV: inventário público inicial

Data: 28/10/2026. Este inventário é **observacional e parcial**. Nenhuma
funcionalidade autenticada foi considerada concluída por aparecer em um bundle
ou em uma rota.

## IEPREV Pro

- URL: `https://app.ieprev.com.br`.
- A raiz redireciona para `/auth`.
- A tela pública tem e-mail, senha, “Esqueceu a senha?”, “Entrar”, “Criar
  conta”, “Continuar cadastro”, Política de Privacidade e Termos de Uso.
- Título: `IEPREV Pro — Plataforma de Inteligência Previdenciária`.
- Não há login Google/Microsoft visível na tela pública observada.
- Rotas candidatas em bundle público já registradas em
  `rea-bundles-publicos-2026-10-28.md`: cálculo de benefício, planejamento
  previdenciário, resultado, relatório, gestão, clientes, assinador e laudos.
  A presença no bundle não prova acesso, conclusão ou contrato.
- O código público sugere uma aplicação Nuxt; a autenticação é obrigatória.

## IEPREV Calc

- URL: `https://calculos.ieprev.com.br`.
- A raiz mostra “Erro (CA2) É necessário estar logado para acessar esta
  página.” sem sessão.
- O bundle público principal e o chunk do importador CNIS foram analisados
  previamente; o importador usa PDF.js, `getTextContent`, concatenção de texto
  por página e uma etapa `newImportText`. Isso é leitura estática, não prova de
  que o fluxo autenticado execute essa sequência.
- Segmentos de rota observados no bundle: `importador-cnis`, `rgps`,
  `contagem-tempo`, `contribuicoes`, `beneficios`, `valores-em-atraso` e
  `transicao`.
- O código do chunk menciona o indicador `PREC-MENOR-MIN`.
- Nenhum cálculo, upload ou relatório autenticado foi observado nesta sessão.

## Bloqueio de sessão

O navegador do conector `chrome-devtools-mcp` já está em execução no perfil
`C:\Users\Ricardo PC\.cache\chrome-devtools-mcp\chrome-profile`. Uma segunda
instância foi recusada; o perfil está bloqueado por outro processo e o arquivo
`Default/Network/Cookies` não pôde ser copiado. Os perfis Chrome normais foram
verificados e não contêm cookies de `ieprev.com.br`. As credenciais não foram
extraídas de gerenciador de senhas.

Consequência: a auditoria autenticada foi executada em uma sessão isolada
posterior. As observações estão em `2026-10-28-sessao-autenticada.md`; a análise
pública não foi tratada como equivalência funcional.

## Próximo experimento

1. Abrir uma sessão de navegador isolada, sem o perfil em uso pelo Codex, e
   autenticar a conta autorizada.
2. No Pro, criar um caso com o CNIS privado e registrar apenas IDs de etapas,
   parâmetros, avisos e contagens; valores e identidade ficam fora do repo.
3. No Calc, repetir o mesmo cenário e exportar os relatórios em área privada.
4. Comparar: vínculos, competências, salários, regras elegíveis, RMI, faltas,
   mensagens e estrutura do PDF.
5. Converter cada diferença em um teste sintético reproduzível e em uma decisão
   de implementação no previdenciar.io.
