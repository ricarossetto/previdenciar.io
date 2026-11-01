# Validação do motor local contra a API do TRF3

Data: 31/10/2026

## Caso sintético

- Nascimento: 1960-01-01
- Sexo: F
- Data de apuração/DIB: 2025-01-31
- Espécie: 42
- Período: 1994-07-01 a 2019-06-30
- 300 competências, salário nominal de R$ 2.000,00
- Sem descarte, sem teto vigente na DER, sem divisor mínimo

## Resultado oficial

Endpoint: `POST https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-rmi`

- Status: `ok`
- Maior RMI: R$ 4.067,11
- Salário de benefício: R$ 5.083,89
- Coeficiente: 0,8
- Divisor: 300
- Competências: 300

## Resultado local

Política nominal (`corrigirSalarios=false`):

- Status: experimental
- RMI: R$ 1.600,00
- Salário de benefício: R$ 2.000,00
- Coeficiente: 0,8

Política corrigida (`corrigirSalarios=true`, referência 2025-01):

- Status: experimental
- RMI: R$ 3.779,69
- Salário de benefício: R$ 4.724,61
- Coeficiente: 0,8

## Conclusão

O resultado **não bate** com a API oficial.

- O modo nominal local não é comparável: a API do TRF3 corrige os salários e retorna R$ 4.067,11.
- Mesmo no modo corrigido, o local fica R$ 287,42 abaixo (cerca de 7,07%).
- A diferença está na política de correção: o motor local usa **INPC em todo o período**. A resposta oficial declara IPC-R (07/1994–06/1995), INPC (07/1995–03/1996), IGP-DI (04/1996–01/2004), INPC (02/2004–01/2008) e INPC a partir de 02/2008.
- Portanto, o motor local não deve ser apresentado como equivalente ao cálculo oficial enquanto a série de correção não for alinhada ou declarada como divergência.

## Próxima correção necessária

1. Implementar a mesma política de índices por período da API oficial.
2. Repetir o caso e exigir igualdade de salário de benefício e RMI ao centavo.
3. Criar casos de regressão para espécies 41 e 42 e para sexos M/F.
4. Manter o modo nominal explicitamente separado do modo de conferência oficial.
## Revisão crítica

A revisão cruzada recomendou um híbrido:

- Corrigir a série de índices para o perfil oficial versionado, sem fingir que um único caso prova equivalência.
- Manter o resultado local e o oficial identificados por fonte enquanto a paridade multi-caso não estiver comprovada.
- Validar fronteiras de índice, série salarial não uniforme, meses extremos e o passo a passo do salário de benefício; não aceitar apenas o total da RMI.
- Não fechar o selo de equivalência com um único caso ou com tolerância de um centavo.

Status: **divergência confirmada; correção ainda não aplicada**.
