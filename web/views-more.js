function viewAssistant() {
  const step = ASSISTANT[state.assistantStep];
  const steps = ASSISTANT.map((s, i) => '<li class="step" data-status="' + (i < state.assistantStep ? 'done' : i === state.assistantStep ? 'current' : 'todo') + '"><span class="step__num">' + String(i + 1).padStart(2, '0') + '</span><span>' + esc(s.title) + '</span></li>').join('');
  const choices = step.choices.length ? '<div class="choices">' + step.choices.map((ch, i) => '<button type="button" class="choice" data-choice="' + i + '" aria-pressed="' + (state.assistantChoice === i) + '"><span class="choice__mark">' + (state.assistantChoice === i ? '✓' : '') + '</span><span>' + esc(ch) + '</span></button>').join('') + '</div>' : '<div class="notice">' + ico('check', 16) + '<p><strong>Etapa conferida.</strong> Nenhum dado é confirmado por nome ou por número fictício.</p></div>';
  const evidence = step.evidence.map((e) => '<span class="chip">' + esc(e) + '</span>').join('');
  return pageHead('Assistente de cálculos', 'Uma entrevista em sete etapas que organiza a hipótese e preserva cada resposta.') +
    '<ol class="steps" aria-label="Etapas do assistente">' + steps + '</ol>' +
    '<div class="case-grid"><section class="folio"><div class="folio__body"><div class="question"><h2>' + esc(step.question) + '</h2><p>' + esc(step.text) + '</p><div class="evidence">' + evidence + '</div>' + choices + '<div class="form-actions"><button type="button" class="btn btn--quiet" data-assistant-back' + (state.assistantStep === 0 ? ' disabled' : '') + '>Voltar</button><button type="button" class="btn btn--primary" data-assistant-next>' + (state.assistantStep === 6 ? 'Concluir conferência' : 'Confirmar e avançar') + ico('arrow', 15) + '</button></div></div></div></section>' +
    '<aside class="side-stack"><section class="folio"><div class="folio__head"><div class="folio__title">Conferência</div></div><div class="folio__body"><dl class="ledger-list" style="margin:0"><div class="ledger-row"><dt>Etapa</dt><dd class="mono">' + (state.assistantStep + 1) + ' de 7</dd></div><div class="ledger-row"><dt>Vínculos</dt><dd class="mono">4</dd></div><div class="ledger-row"><dt>Competências</dt><dd class="mono">431</dd></div><div class="ledger-row"><dt>Pendências</dt><dd class="mono" style="color:var(--gold)">1</dd></div></dl><p class="form-note">O assistente pergunta, importa e confere. Ele não decide a regra nem inventa dados.</p></div></section><div class="notice">' + ico('lock', 16) + '<p><strong>Entrevista local.</strong> Nenhuma resposta desta demonstração é enviada a modelo ou servidor.</p></div></aside></div>';
}

function viewEditor() {
  return pageHead('Editor de dados', 'Ajuste períodos, salários e parâmetros. Cada alteração invalida o resultado anterior.', '<button type="button" class="btn btn--quiet" data-toast="Rascunho salvo na demonstração.">Salvar rascunho</button><button type="button" class="btn btn--primary" data-toast="Recálculo simulado. Na versão desktop, o motor Rust executa a memória.">Recalcular</button>') +
    '<div class="case-grid"><div class="stack"><section class="folio"><div class="folio__head"><div><div class="folio__title">Identificação e período</div><div class="folio__sub">Caso de Helena Fagundes · RGPS</div></div><span class="state state--warn">Resultado desatualizado</span></div><div class="folio__body"><div class="form-grid">' +
    '<div class="field"><label for="f-name">Nome do cálculo</label><input class="input" id="f-name" value="Aposentadoria por tempo de contribuição"></div>' +
    '<div class="field"><label for="f-client">Cliente</label><input class="input" id="f-client" value="Helena Fagundes"></div>' +
    '<div class="field"><label for="f-start">Início do primeiro vínculo</label><input class="input" id="f-start" value="03/1990"></div>' +
    '<div class="field"><label for="f-end">Fim do último vínculo</label><input class="input" id="f-end" value="06/2026"></div>' +
    '<div class="field field--wide field--error"><label for="f-dib">DIB</label><input class="input" id="f-dib" value="" aria-describedby="f-dib-error" aria-invalid="true"><span class="field__error" id="f-dib-error">Informe a data de início do benefício para recalcular.</span></div>' +
    '</div></div></section>' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Salários e parâmetros</div></div><div class="folio__body"><div class="form-grid">' +
    '<div class="field"><label for="f-last">Última remuneração</label><input class="input" id="f-last" value="4.120,00"></div>' +
    '<div class="field"><label for="f-comp">Competência</label><input class="input" id="f-comp" value="05/2026"></div>' +
    '<div class="field"><label for="f-rule">Regra testada</label><select class="input" id="f-rule"><option>Tempo de contribuição</option><option>Pontos</option><option>Pedágio 50%</option><option>Idade progressiva</option></select></div>' +
    '<div class="field"><label for="f-special">Tempo especial</label><select class="input" id="f-special"><option>Não considerado</option><option>Considerar com PPP</option></select></div>' +
    '</div><p class="form-note">O campo DIB está vazio de propósito para mostrar o estado de erro. Alterar qualquer campo invalida o resultado e a revisão anterior.</p></div></section>' +
    '<div class="notice notice--bad">' + ico('alert', 16) + '<p><strong>Resultado desatualizado.</strong> Salve e recalcule para gerar uma nova memória; o relatório anterior não é sobrescrito.</p></div></div>' +
    '<aside class="side-stack"><section class="folio"><div class="folio__head"><div class="folio__title">Última revisão</div></div><div class="folio__body"><dl class="ledger-list" style="margin:0"><div class="ledger-row"><dt>Data</dt><dd class="mono">24/10/2026</dd></div><div class="ledger-row"><dt>RMI</dt><dd class="gold">R$ 4.812,37</dd></div><div class="ledger-row"><dt>Pendências</dt><dd class="mono">2</dd></div></dl><p class="form-note">A revisão anterior fica no histórico; nada é apagado.</p></div></section></aside></div>';
}

function viewTables() {
  const s = SERIES.find((x) => x.id === state.seriesId) || SERIES[0];
  const list = SERIES.map((x) => '<button type="button" class="list__item" data-series="' + x.id + '" aria-current="' + (x.id === state.seriesId) + '"><strong>' + esc(x.name) + '</strong><span>' + esc(x.unit) + ' — ' + esc(x.periodicity) + ' — ' + x.entries.length + ' períodos</span></button>').join('');
  const rows = s.entries.map((e) => '<tr><td class="mono">' + esc(e.period) + '</td><td class="num">' + esc(e.value) + '</td><td class="muted">' + esc(e.note) + '</td></tr>').join('');
  return pageHead('Tabelas e índices', 'Séries manuais com fonte e revisão declaradas. Não há download nem validação implícita de índice oficial.') +
    '<div class="split"><section class="folio"><div class="folio__head"><div class="folio__title">Séries</div></div><div class="folio__body folio__body--flush"><div class="list">' + list + '</div></div></section>' +
    '<section class="folio"><div class="folio__head"><div><div class="folio__title">' + esc(s.name) + '</div><div class="folio__sub">' + esc(s.provenance) + '</div></div>' + (s.verified ? '<span class="state state--ok">Conferida</span>' : '<span class="state state--warn">Não certificada</span>') + '</div><div class="folio__body folio__body--flush"><div class="table-wrap"><table class="table"><thead><tr><th>Período</th><th class="num">Valor</th><th>Observação</th></tr></thead><tbody>' + rows + '</tbody></table></div></div></section></div>' +
    '<div class="notice" style="margin-top:14px">' + ico('alert', 16) + '<p><strong>Valores de demonstração.</strong> Confira a fonte oficial antes de usar qualquer série em um cálculo real. Rótulos não contêm coeficientes.</p></div>';
}

function viewModules() {
  const rows = MODULES.map((m) => '<div class="module-row"><div class="module-row__name">' + esc(m.name) + '</div><div class="module-row__desc">' + esc(m.desc) + '</div><div class="inline-actions">' + stateTag(m.status, 'quiet') + '<button type="button" class="btn btn--ghost" data-toast="Este módulo não está implementado. O limite está documentado no repositório.">Ver limite</button></div></div>').join('');
  return pageHead('Módulos de apoio', 'Planejamento, alíquota, IRRF e retorno. O que não está pronto aparece como não pronto.') +
    '<section class="folio"><div class="folio__head"><div><div class="folio__title">Módulos previstos</div><div class="folio__sub">Nenhum deles é usado pelo cálculo principal nesta versão.</div></div></div><div class="folio__body">' + rows + '</div></section>' +
    '<div class="notice" style="margin-top:14px">' + ico('alert', 16) + '<p><strong>Fora do escopo atual.</strong> A ausência é declarada; o produto não simula alíquota, IRRF ou retorno como se estivessem prontos.</p></div>';
}

function viewDocuments() {
  if (!DOCUMENTS.length) {
    return pageHead('Documentos', 'Modelos locais com campos substituíveis. Nenhum documento de cliente entra como exemplo.', '<button type="button" class="btn btn--primary" data-toast="Modelo novo simulado. A biblioteca real fica na base local.">' + ico('plus', 15) + 'Novo modelo</button>') + '<section class="folio"><div class="folio__body"><div class="empty"><strong>Nenhum modelo no workspace</strong><span>Importe um JSON com documentos ou crie um modelo na base local.</span></div></div></section>';
  }
  const d = DOCUMENTS.find((x) => x.id === state.docId) || DOCUMENTS[0];
  const list = DOCUMENTS.map((x) => '<button type="button" class="list__item" data-doc="' + x.id + '" aria-current="' + (x.id === state.docId) + '"><strong>' + esc(x.name) + '</strong><span>' + esc(x.category) + ' — atualizado em ' + dateBR(x.updated) + '</span></button>').join('');
  const chips = ['{{nome}}', '{{cpf}}', '{{nascimento}}', '{{calculo}}', '{{dib}}', '{{escritorio}}'].map((c) => '<span class="chip">' + c + '</span>').join('');
  return pageHead('Documentos', 'Modelos locais com campos substituíveis. Nenhuma petição proprietária ou documento de cliente entra como exemplo.', '<button type="button" class="btn btn--primary" data-toast="Modelo novo simulado. A biblioteca real fica na base local.">' + ico('plus', 15) + 'Novo modelo</button>') +
    '<div class="split"><section class="folio"><div class="folio__head"><div class="folio__title">Biblioteca</div></div><div class="folio__body folio__body--flush"><div class="list">' + list + '</div></div></section>' +
    '<section class="folio"><div class="folio__head"><div><div class="folio__title">' + esc(d.name) + '</div><div class="folio__sub">' + esc(d.category) + '</div></div><span class="state state--quiet">Modelo</span></div><div class="folio__body"><p style="margin:0;color:var(--ink-2);font-size:13px">' + esc(d.body) + '</p><div class="evidence" style="margin-top:12px">' + chips + '</div><p class="form-note">Campos entre chaves são substituídos no momento de gerar o texto. Conteúdo não é executado como instrução.</p><div class="form-actions"><button type="button" class="btn btn--quiet" data-toast="Editor de modelo simulado.">Editar</button><button type="button" class="btn btn--primary" data-toast="Documento preenchido na demonstração. Nenhum arquivo foi gravado.">Gerar texto</button></div></div></section></div>';
}
