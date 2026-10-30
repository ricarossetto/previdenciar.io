# Contratos de evidência e leitura

Este documento descreve a camada `source_documents` / `readings` / `facts` do
workspace. Ela é a base para as ferramentas previdenciárias que dependem de
documento, página e trecho.

## Persistência

O workspace passou ao schema 3. A migração do schema 2 apenas acrescenta
vetores vazios; nenhum cálculo, identidade ou valor existente é reescrito.

```text
source_documents: Documento[]
readings: Leitura[]
facts: Fato[]
```

O PDF binário não é gravado. O texto extraído, quando necessário para revisão,
fica em `Leitura.texto_extraido`, dentro do workspace cifrado por DPAPI, com
limite de 200 mil caracteres por leitura.

## Tipos

### Documento

```text
id, nome_arquivo, mime, paginas, origem_texto ("nativo" | "ocr"),
ocr_confianca (0..1 ou nulo), hash_sha256 (64 hex), criado_em
```

### Leitura

```text
id, documento_id, tipo, campos[], paginas_lidas, pendencias[],
status ("em_revisao" | "confirmada"), texto_extraido
```

Tipos aceitos: `cnis`, `laudo_medico`, `processo_administrativo`, `ppp`,
`prova_rural`, `grupo_familiar`, `carta_de_concessao`,
`documentos_da_pensao`, `documento_generico`.

### CampoExtraido

```text
chave, valor (escalar ou nulo), confianca ("alta" | "media" | "baixa"),
origem { documento_id, pagina (1-based), trecho (até 500 caracteres) }
```

### Fato

```text
id, texto, proveniencia ("comprovado" | "alegado" | "inferido"),
origem { documento_id, pagina, trecho }, alegado_por, confirmado, usado_em[]
```

`comprovado` exige origem. `alegado` exige `alegado_por`. `inferido` só pode ser
usado por cálculo depois de `confirmado = true`.

### Pendência e alerta

```text
Pendencia { campo, motivo, pergunta }
Alerta { codigo, severidade ("critico" | "atencao" | "info"), mensagem }
```

## Regras de validação

- IDs de documento, leitura e fato são únicos, não vazios e limitados a 80
  caracteres.
- Toda leitura e todo fato apontam para documento existente.
- Página de origem é 1-based e não excede o total do documento.
- Campo extraído exige chave única por leitura e valor escalar ou nulo.
- Fato alegado exige `alegado_por`.
- Cálculo só pode referenciar leitura `confirmada`.
- Cálculo não pode usar fato `inferido` sem confirmação humana.
- Referências a cálculo em `usado_em` precisam existir.

## Limites atuais

- A interface da Minuta ainda salva uma leitura genérica do anexo inteiro;
  extração campo a campo virá nas ferramentas de laudo, PPP, processo e rural.
- O texto extraído é limitado e não substitui o PDF original.
- A camada não decide verdade, vigência, direito ou mérito.
