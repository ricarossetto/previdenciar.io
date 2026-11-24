const FABRICA_TABS = [
  { id: 'tempo', label: 'Tempo de contribuição' },
  { id: 'rmi', label: 'Renda mensal (RMI)' },
  { id: 'atrasados', label: 'Atrasados' },
  { id: 'documentos', label: 'Documentos' },
  { id: 'indices', label: 'Índices e limites' },
  { id: 'referencias', label: 'Referências' },
  { id: 'console', label: 'Console da API' }
];

const FABRICA_ESPECIES = [
  [21, '21 · Pensão por morte'],
  [25, '25 · Auxílio-reclusão'],
  [31, '31 · Auxílio por incapacidade temporária'],
  [32, '32 · Aposentadoria por incapacidade permanente'],
  [36, '36 · Auxílio-acidente'],
  [41, '41 · Aposentadoria por idade'],
  [42, '42 · Aposentadoria por tempo de contribuição'],
  [46, '46 · Aposentadoria especial'],
  [57, '57 · Aposentadoria especial (professor)'],
  [80, '80 · Salário-maternidade'],
  [87, '87 · Amparo social à pessoa com deficiência'],
  [88, '88 · Amparo social ao idoso'],
  [91, '91 · Auxílio por incapacidade temporária (acidentário)'],
  [92, '92 · Aposentadoria por incapacidade permanente (acidentária)'],
  [93, '93 · Pensão por morte (acidentária)'],
  [94, '94 · Auxílio-acidente (acidentário)'],
  [95, '95 · Auxílio-acidente (previdenciário)']
];

const FABRICA_FORMAS = [
  ['COMUM', 'Comum'],
  ['ESPECIAL_25', 'Especial 25 anos'],
  ['ESPECIAL_20', 'Especial 20 anos'],
  ['ESPECIAL_15', 'Especial 15 anos'],
  ['RURAL', 'Rural'],
  ['MAGISTERIO', 'Magistério'],
  ['NAO_COMPUTAR', 'Não computar']
];

const FABRICA_NATUREZAS = [
  ['EMPREGADO', 'Empregado'],
  ['EMPREGADO_DOMESTICO', 'Empregado doméstico'],
  ['CONTRIBUINTE_INDIVIDUAL', 'Contribuinte individual'],
  ['TRABALHADOR_AVULSO', 'Trabalhador avulso'],
  ['SEGURADO_ESPECIAL', 'Segurado especial'],
  ['SEGURADO_FACULTATIVO', 'Segurado facultativo'],
  ['LC_123_CONTRIBUINTE_INDIVIDUAL', 'LC 123 · Contribuinte individual'],
  ['LC_123_FACULTATIVO', 'LC 123 · Facultativo'],
  ['BENEFICIO_INCAPACIDADE', 'Benefício por incapacidade'],
  ['AUXILIO_ACIDENTE', 'Auxílio-acidente'],
  ['OUTROS_BENEFICIOS_INSS', 'Outros benefícios do INSS'],
  ['SALARIO_MATERNIDADE', 'Salário-maternidade'],
  ['RECLUSO', 'Recluso'],
  ['SERVICO_MILITAR', 'Serviço militar'],
  ['CONTRIB_INDIVIDUAL_PRESTADOR_SERVICO', 'Contribuinte individual prestador de serviço']
];

const FABRICA_MARCOS_CARENCIA = [
  ['IMPLEMENTO_IDADE', 'Implemento de idade'],
  ['DER', 'DER']
];

const FABRICA_MARCOS_RMI_PADRAO = [
  ['NA_DIB', 'DIB na DER'],
  ['DIREITO_ADQUIRIDO_ANTES_EC_20', 'Direito adquirido antes da EC 20/98'],
  ['DIREITO_ADQUIRIDO_ANTES_EC_103', 'Direito adquirido antes da EC 103/2019'],
  ['NA_MUDANCA_DAS_REGRAS_PROGRESSIVAS', 'Mudança das regras progressivas'],
  ['NA_LEI_9876', 'Lei 9.876/99'],
  ['NA_REGRA_DE_PONTOS_PARA_FATOR', 'Regra de pontos para o fator'],
  ['NO_RESTABELECIMENTO_DO_DIVISOR_MINIMO', 'Restabelecimento do divisor mínimo'],
  ['NA_MUDANCA_DAS_REGRAS_PROGRESSIVAS_SEM_DIVISOR_MINIMO', 'Regras progressivas sem divisor mínimo']
];

const FAB = {
  tab: 'tempo',
  health: null,
  fundamentos: null,
  marcos: null,
  docsResult: null,
  indexResult: null,
  refBcb: null,
  refIbge: null,
  refCjf: null,
  refCjfLoaded: false,
  consoleResult: null
};

let fabWired = false;

function fabToday() {
  return new Date().toISOString().slice(0, 10);
}

function fabMoneyNumber(value) {
  if (value == null) return null;
  let text = String(value).replace(/[^\d.,-]/g, '');
  if (!text) return null;
  if (text.indexOf(',') >= 0) text = text.replace(/\./g, '').replace(',', '.');
  const number = Number(text);
  return Number.isFinite(number) ? number : null;
}

function fabMoneyText(value) {
  if (value == null || value === '') return '';
  return String(value).replace('.', ',');
}

function fabNumber(value) {
  const number = fabMoneyNumber(value);
  return number == null ? null : number;
}

function fabEndOfMonth(ym) {
  const match = String(ym || '').match(/^(\d{4})-(\d{2})$/);
  if (!match) return '';
  const date = new Date(Number(match[1]), Number(match[2]), 0);
  return date.getFullYear() + '-' + String(date.getMonth() + 1).padStart(2, '0') + '-' + String(date.getDate()).padStart(2, '0');
}

function fabDefaultData(mode) {
  return {
    dataApuracao: fabToday(),
    der: '',
    dadosSegurado: { nascimento: '1968-02-14', sexo: 'F', tipo: 'COMUM' },
    opcoesContagem: { especie: 42 },
    periodosContribuicao: [{
      dataInicial: '2010-01-01',
      dataFinal: '2024-12-31',
      descricao: 'Vínculo principal',
      formaContagem: 'COMUM',
      naturezaVinculo: 'EMPREGADO',
      contarCarencia: true,
      prioritario: false,
      salarios: [{ competencia: '2010-01', valor: 3000 }]
    }],
    fundamento: 'EC_103_ART_20',
    marcoTemporal: 'NA_DIB',
    aplicarIrsmFev94: false
  };
}

function fabDataFromCase() {
  const current = caseById(state.caseId);
  const client = clientById(current.client);
  const periods = (PERIODS[current.id] || []).slice();
  const contributions = contribsFor(current);
  const data = fabDefaultData('tempo');
  data.dataApuracao = current.dib || fabToday();
  data.dadosSegurado = {
    nascimento: client.birth || '',
    sexo: /^m/i.test(client.sex || '') ? 'M' : 'F',
    tipo: 'COMUM'
  };
  if (!periods.length) {
    periods.push({ employer: current.name, kind: 'Empregado', start: current.start, end: current.end, status: 'conferido' });
  }
  data.periodosContribuicao = periods.map((period) => {
    const kind = String(period.kind || '');
    const forma = /especial/i.test(kind) ? 'ESPECIAL_25' : /rural/i.test(kind) ? 'RURAL' : 'COMUM';
    const natureza = /aut[oô]nomo|individual/i.test(kind) ? 'CONTRIBUINTE_INDIVIDUAL' : /dom[eé]stico/i.test(kind) ? 'EMPREGADO_DOMESTICO' : 'EMPREGADO';
    const salarios = contributions
      .filter((row) => row.comp >= period.start && row.comp <= period.end)
      .filter((row) => row.status !== 'excluido' && row.status !== 'pendente')
      .map((row) => ({ competencia: row.comp, valor: fabMoneyNumber(row.value) }))
      .filter((row) => row.competencia && row.valor != null);
    return {
      dataInicial: period.start + '-01',
      dataFinal: fabEndOfMonth(period.end),
      descricao: period.employer,
      formaContagem: forma,
      naturezaVinculo: natureza,
      contarCarencia: period.status !== 'excluido' && period.status !== 'pendente',
      prioritario: false,
      salarios
    };
  });
  return data;
}

function fabSalariosText(salarios) {
  if (!Array.isArray(salarios)) return '';
  return salarios.map((item) => String(item.competencia || '') + ';' + fabMoneyText(item.valor)).join('\n');
}

function fabParseSalarios(text) {
  const rows = [];
  String(text || '').split(/\r?\n/).forEach((line, index) => {
    const clean = line.trim();
    if (!clean || clean.startsWith('#')) return;
    const parts = clean.split(/[;\t]/).map((part) => part.trim());
    if (parts.length < 2) throw new Error('Salário na linha ' + (index + 1) + ': use AAAA-MM;valor.');
    if (!/^\d{4}-\d{2}$/.test(parts[0])) throw new Error('Competência inválida na linha ' + (index + 1) + ': ' + parts[0] + '.');
    const value = fabNumber(parts[1]);
    if (value == null) throw new Error('Valor inválido na linha ' + (index + 1) + ': ' + parts[1] + '.');
    rows.push({ competencia: parts[0], valor: value });
  });
  return rows;
}

function fabOptions(items, selected) {
  return items.map((item) => '<option value="' + esc(item[0]) + '"' + (String(item[0]) === String(selected) ? ' selected' : '') + '>' + esc(item[1]) + '</option>').join('');
}

function fabFundamentoOptions(selected) {
  const list = FAB.fundamentos && Array.isArray(FAB.fundamentos.result)
    ? FAB.fundamentos.result.map((item) => [item.valor, item.rotulo])
    : [['EC_103_ART_20', 'EC 103, art. 20']];
  return fabOptions(list, selected || 'EC_103_ART_20');
}

function fabMarcoOptions(selected) {
  const list = FAB.marcos && Array.isArray(FAB.marcos.result)
    ? FAB.marcos.result.map((item) => [item.valor, item.rotulo])
    : FABRICA_MARCOS_RMI_PADRAO;
  return fabOptions(list, selected || 'NA_DIB');
}

function fabPeriodRow(prefix, index, period) {
  period = period || {};
  const salarios = Array.isArray(period.salarios) ? fabSalariosText(period.salarios) : '';
  return '<fieldset class="fab-period" data-fab-period>' +
    '<legend>Período <span class="mono" data-fab-period-number>' + (index + 1) + '</span></legend>' +
    '<div class="form-grid">' +
    '<div class="field"><label>Início</label><input class="input" type="date" data-field="dataInicial" aria-label="Data inicial do período" value="' + esc(period.dataInicial || '') + '"></div>' +
    '<div class="field"><label>Fim</label><input class="input" type="date" data-field="dataFinal" aria-label="Data final do período" value="' + esc(period.dataFinal || '') + '"></div>' +
    '<div class="field field--wide"><label>Descrição</label><input class="input" type="text" data-field="descricao" aria-label="Descrição do período" maxlength="200" value="' + esc(period.descricao || '') + '" placeholder="Empresa, vínculo ou período"></div>' +
    '<div class="field"><label>Forma de contagem</label><select class="input" data-field="formaContagem" aria-label="Forma de contagem">' + fabOptions(FABRICA_FORMAS, period.formaContagem || 'COMUM') + '</select></div>' +
    '<div class="field"><label>Natureza do vínculo</label><select class="input" data-field="naturezaVinculo" aria-label="Natureza do vínculo">' + fabOptions(FABRICA_NATUREZAS, period.naturezaVinculo || 'EMPREGADO') + '</select></div>' +
    '<div class="field field--wide"><label>Salários de contribuição</label><textarea class="textarea fab-code fab-code--short" data-field="salarios" aria-label="Salários de contribuição, uma competência por linha" rows="4" spellcheck="false" placeholder="2010-01;3000,00&#10;2010-02;3000,00">' + esc(salarios) + '</textarea><span class="field__hint">Uma competência por linha, no formato AAAA-MM;valor. Sem salários, o período ainda pode contar tempo e carência.</span></div>' +
    '<label class="fab-check"><input type="checkbox" data-field="contarCarencia"' + (period.contarCarencia !== false ? ' checked' : '') + '> Contar para carência</label>' +
    '<label class="fab-check"><input type="checkbox" data-field="prioritario"' + (period.prioritario ? ' checked' : '') + '> Prioritário na concomitância</label>' +
    '</div>' +
    '<div class="fab-period__foot"><button type="button" class="btn btn--ghost" data-fab-remove-period="' + esc(prefix) + '">Remover período</button></div>' +
    '</fieldset>';
}

function fabPeriodsHtml(prefix, periods) {
  const list = Array.isArray(periods) && periods.length ? periods : [{}];
  return list.map((period, index) => fabPeriodRow(prefix, index, period)).join('');
}

function fabCalcPanel(prefix, mode) {
  const isRmi = mode === 'rmi';
  const data = fabDefaultData(mode);
  const title = isRmi ? 'Renda mensal inicial (RMI)' : 'Tempo de contribuição';
  const subtitle = isRmi
    ? 'Calcula a RMI por fundamento com o mesmo payload de tempo. A resposta traz os marcos e os benefícios mais vantajosos.'
    : 'Calcula o tempo, a carência e os benefícios cabíveis. Inclua os salários quando quiser apurar carência e RMI.';
  return '<section class="folio"><div class="folio__head"><div><div class="folio__title">' + title + '</div><div class="folio__sub">' + subtitle + '</div></div><span class="state state--quiet">POST ' + (isRmi ? '/calculos-rmi/por-fundamento' : '/calculos-tempo') + '</span></div><div class="folio__body">' +
    '<form id="fab-form-' + prefix + '" data-fab-form="' + prefix + '" data-fab-mode="' + mode + '" novalidate>' +
    '<div class="form-section"><div class="form-section__title">Segurado e apuração</div><div class="form-grid">' +
    '<div class="field"><label for="fab-' + prefix + '-apuracao">Data de apuração (DIB)</label><input class="input" id="fab-' + prefix + '-apuracao" type="date" data-field="dataApuracao" value="' + esc(data.dataApuracao) + '"></div>' +
    '<div class="field"><label for="fab-' + prefix + '-der">DER (opcional)</label><input class="input" id="fab-' + prefix + '-der" type="date" data-field="der" value=""></div>' +
    '<div class="field"><label for="fab-' + prefix + '-nascimento">Nascimento</label><input class="input" id="fab-' + prefix + '-nascimento" type="date" data-field="nascimento" value="' + esc(data.dadosSegurado.nascimento) + '"></div>' +
    '<div class="field"><label for="fab-' + prefix + '-sexo">Sexo</label><select class="input" id="fab-' + prefix + '-sexo" data-field="sexo"><option value="F"' + (data.dadosSegurado.sexo === 'F' ? ' selected' : '') + '>Feminino</option><option value="M"' + (data.dadosSegurado.sexo === 'M' ? ' selected' : '') + '>Masculino</option></select></div>' +
    '<div class="field"><label for="fab-' + prefix + '-tipo">Tipo de segurado</label><select class="input" id="fab-' + prefix + '-tipo" data-field="tipo"><option value="COMUM"' + (data.dadosSegurado.tipo === 'COMUM' ? ' selected' : '') + '>Comum</option><option value="PCD"' + (data.dadosSegurado.tipo === 'PCD' ? ' selected' : '') + '>Pessoa com deficiência</option><option value="RURICOLA"' + (data.dadosSegurado.tipo === 'RURICOLA' ? ' selected' : '') + '>Rurícola</option></select></div>' +
    '<div class="field"><label for="fab-' + prefix + '-especie">Espécie</label><select class="input" id="fab-' + prefix + '-especie" data-field="especie">' + fabOptions(FABRICA_ESPECIES, data.opcoesContagem.especie) + '</select></div>' +
    '</div></div>' +
    (isRmi ? '<div class="form-section"><div class="form-section__title">RMI</div><div class="form-grid">' +
      '<div class="field"><label for="fab-' + prefix + '-fundamento">Fundamento</label><select class="input" id="fab-' + prefix + '-fundamento" data-field="fundamento">' + fabFundamentoOptions(data.fundamento) + '</select><span class="field__hint">Lista carregada de /fundamentos.</span></div>' +
      '<div class="field"><label for="fab-' + prefix + '-marco">Marco temporal</label><select class="input" id="fab-' + prefix + '-marco" data-field="marcoTemporal">' + fabMarcoOptions(data.marcoTemporal) + '</select><span class="field__hint">Lista carregada de /marcos-temporais.</span></div>' +
      '<label class="fab-check"><input type="checkbox" data-field="aplicarIrsmFev94"' + (data.aplicarIrsmFev94 ? ' checked' : '') + '> Aplicar IRSM fev/94</label>' +
      '</div></div>' : '') +
    '<div class="form-section"><div class="fab-section-head"><div><div class="form-section__title">Períodos de contribuição</div><span class="field__hint">Os períodos e salários são enviados exatamente como preenchidos; campos vazios não são inventados.</span></div><button type="button" class="btn btn--quiet" data-fab-add-period="' + prefix + '">Adicionar período</button></div>' +
    '<div class="fab-periods" data-fab-periods="' + prefix + '">' + fabPeriodsHtml(prefix, data.periodosContribuicao) + '</div></div>' +
    '<div class="form-actions"><button type="button" class="btn btn--ghost" data-fab-use-case="' + prefix + '">Usar caso atual</button><button type="submit" class="btn btn--primary">' + (isRmi ? 'Calcular RMI' : 'Calcular tempo') + '</button></div>' +
    '<p class="form-note">A API devolve o envelope PDPJ. Em caso de 400, as mensagens de validação são mostradas abaixo, sem esconder o erro.</p>' +
    '</form><div id="fab-result-' + prefix + '" class="fab-result" aria-live="polite"></div></div></section>';
}

function fabErrorHtml(error) {
  const payload = error && error.payload;
  const messages = payload && Array.isArray(payload.messages) ? payload.messages : [];
  const list = messages.length ? messages : [error && error.message ? error.message : 'Falha desconhecida.'];
  return '<div class="notice notice--bad">' + ico('alert', 16) + '<div><strong>Não foi possível concluir.</strong><ul class="fab-error-list">' +
    list.map((message) => '<li>' + esc(message) + '</li>').join('') +
    '</ul>' + (error && error.status === 0 ? '<p class="form-note">Inicie o servidor local: <span class="mono">node server/index.js</span>. A chave fica no <span class="mono">.env</span> do servidor.</p>' : '') +
    '</div></div>' + (payload ? '<details class="fab-raw"><summary>Envelope de erro</summary><pre class="json">' + esc(JSON.stringify(payload, null, 2)) + '</pre></details>' : '');
}

function fabDias(value) {
  const days = Number(value);
  if (!Number.isFinite(days)) return '—';
  return days + ' dias';
}

function fabValor(value) {
  const number = Number(value);
  return Number.isFinite(number) ? brl(number) : '—';
}

function fabRequisitosHtml(benefit) {
  const na = benefit && benefit.dadosApuracao && benefit.dadosApuracao.naApuracao;
  const values = (na && na.valoresApurados) || {};
  const rows = Object.keys(values).map((key) => {
    const item = values[key] || {};
    const result = item.resultado || {};
    return '<div class="ledger-row"><dt>' + esc(item.rotulo || key) + '</dt><dd>' +
      (result.cumprido ? stateTag('cumprido', 'ok') : stateTag('não cumprido', 'quiet')) +
      '<span class="fab-requisito-text">' + esc(result.porExtenso || '') + '</span></dd></div>';
  }).join('');
  const data = na && na.data ? '<div class="fab-benefit__data">Apurado em ' + dateBR(na.data) + '</div>' : '';
  return data + (rows ? '<div class="ledger-list">' + rows + '</div>' : '<p class="muted">Sem requisitos detalhados neste bloco.</p>');
}

function fabBenefitHtml(benefit, tone, label) {
  const desc = benefit.descricao || 'Benefício';
  const fund = benefit.fundamento || benefit.chaveFundamento || '';
  return '<details class="fab-benefit"' + (tone === 'ok' ? ' open' : '') + '>' +
    '<summary><span class="state state--' + tone + '">' + esc(label) + '</span><strong>' + esc(desc) + '</strong><span class="muted">' + esc(fund) + '</span></summary>' +
    '<div class="fab-benefit__body">' + fabRequisitosHtml(benefit) + '</div></details>';
}

function fabRenderTempo(payload) {
  if (!payload || payload.status !== 'ok' || !payload.result) return fabErrorHtml({ message: 'A Fábrica não devolveu um resultado de tempo.', payload });
  const result = payload.result;
  const totais = result.totais || {};
  const cumpridos = (result.beneficios && result.beneficios.cumpridos) || [];
  const naoCumpridos = (result.beneficios && result.beneficios.naoCumpridos) || [];
  return '<div class="fab-metrics">' +
    '<div class="fab-metric"><span>Tempo simples</span><strong>' + esc(fabDias(totais.simples)) + '</strong><small>' + esc(totais.rotulo || '') + '</small></div>' +
    '<div class="fab-metric"><span>Tempo convertido</span><strong>' + esc(fabDias(totais.convertido)) + '</strong><small>Conversão especial quando houver</small></div>' +
    '<div class="fab-metric"><span>Carência</span><strong>' + esc(totais.carencia == null ? '—' : totais.carencia + ' meses') + '</strong><small>' + esc(totais.rotulo || '') + '</small></div>' +
    '<div class="fab-metric"><span>Idade</span><strong>' + esc(totais.idade == null ? '—' : totais.idade + ' anos') + '</strong><small>' + esc(totais.rotulo || '') + '</small></div>' +
    '</div>' +
    '<div class="fab-benefit-group"><div class="folio__title">Benefícios com requisitos cumpridos</div>' + (cumpridos.length ? cumpridos.map((benefit) => fabBenefitHtml(benefit, 'ok', 'cumprido')).join('') : '<p class="muted">Nenhum benefício cumprido com os dados informados.</p>') + '</div>' +
    '<div class="fab-benefit-group"><div class="folio__title">Benefícios não cumpridos</div>' + (naoCumpridos.length ? naoCumpridos.map((benefit) => fabBenefitHtml(benefit, 'quiet', 'não cumprido')).join('') : '<p class="muted">Nenhum benefício analisado.</p>') + '</div>' +
    '<details class="fab-raw"><summary>JSON completo de tempo</summary><pre class="json">' + esc(JSON.stringify(payload, null, 2)) + '</pre></details>';
}

function fabRenderRmi(payload) {
  if (!payload || payload.status !== 'ok') return fabErrorHtml({ message: 'A Fábrica não devolveu um resultado de RMI.', payload });
  const items = Array.isArray(payload.result) ? payload.result : payload.result ? [payload.result] : [];
  if (!items.length) return '<div class="empty"><strong>Sem blocos de RMI</strong><span>A resposta veio vazia.</span></div><details class="fab-raw"><summary>JSON completo</summary><pre class="json">' + esc(JSON.stringify(payload, null, 2)) + '</pre></details>';
  const blocks = items.map((item) => {
    const values = Array.isArray(item.beneficiosMaisVantajosos) ? item.beneficiosMaisVantajosos : [];
    const conclusion = item.conclusao && item.conclusao.completo === false
      ? '<span class="state state--warn">resultado parcial</span>'
      : '<span class="state state--quiet">completo</span>';
    return '<section class="folio fab-rmi-block"><div class="folio__head"><div><div class="folio__title">' + esc(item.marcoTemporal || 'Marco temporal') + '</div><div class="folio__sub">Maior RMI: ' + esc(fabValor(item.maiorRmi)) + '</div></div>' + conclusion + '</div><div class="folio__body">' +
      (values.length ? values.map((benefit) => '<div class="fab-rmi-line"><strong>' + esc(benefit.descricao || benefit.fundamento || 'Benefício') + '</strong><span class="mono">' + esc(fabValor(benefit.rmi)) + '</span></div>').join('') : '<p class="muted">Nenhum benefício mais vantajoso neste marco.</p>') +
      '</div></section>';
  }).join('');
  return blocks + '<details class="fab-raw"><summary>JSON completo de RMI</summary><pre class="json">' + esc(JSON.stringify(payload, null, 2)) + '</pre></details>';
}

function fabRenderGeneric(payload) {
  const rows = payload && Array.isArray(payload.result) ? payload.result : null;
  if (rows && rows.length && rows[0] && typeof rows[0] === 'object' && !Array.isArray(rows[0])) {
    const keys = Object.keys(rows[0]).slice(0, 8);
    return '<div class="table-wrap"><table class="table"><thead><tr>' + keys.map((key) => '<th>' + esc(key) + '</th>').join('') + '</tr></thead><tbody>' +
      rows.slice(0, 200).map((row) => '<tr>' + keys.map((key) => '<td>' + esc(row[key] == null ? '—' : typeof row[key] === 'object' ? JSON.stringify(row[key]) : row[key]) + '</td>').join('') + '</tr>').join('') +
      '</tbody></table></div><p class="form-note">' + rows.length + ' linha(s) retornada(s). JSON completo abaixo.</p><details class="fab-raw"><summary>JSON completo</summary><pre class="json">' + esc(JSON.stringify(payload, null, 2)) + '</pre></details>';
  }
  return '<pre class="json">' + esc(JSON.stringify(payload, null, 2)) + '</pre>';
}

function fabReadForm(prefix) {
  const form = document.getElementById('fab-form-' + prefix);
  if (!form) throw new Error('Formulário não encontrado.');
  const value = (name) => {
    const field = form.querySelector('[data-field="' + name + '"]');
    return field ? String(field.value || '').trim() : '';
  };
  const checked = (name) => {
    const field = form.querySelector('[data-field="' + name + '"]');
    return Boolean(field && field.checked);
  };
  const dataApuracao = value('dataApuracao');
  const nascimento = value('nascimento');
  if (!dataApuracao) throw new Error('Informe a data de apuração.');
  if (!nascimento) throw new Error('Informe a data de nascimento.');
  const periods = $$('[data-fab-period]', form).map((row, index) => {
    const field = (name) => {
      const input = row.querySelector('[data-field="' + name + '"]');
      return input ? String(input.value || '').trim() : '';
    };
    const check = (name) => {
      const input = row.querySelector('[data-field="' + name + '"]');
      return Boolean(input && input.checked);
    };
    const dataInicial = field('dataInicial');
    const dataFinal = field('dataFinal');
    if (!dataInicial || !dataFinal) throw new Error('Período ' + (index + 1) + ': informe as datas inicial e final.');
    const period = {
      dataInicial,
      dataFinal,
      descricao: field('descricao') || 'Período ' + (index + 1),
      formaContagem: field('formaContagem') || 'COMUM',
      naturezaVinculo: field('naturezaVinculo') || 'EMPREGADO',
      contarCarencia: check('contarCarencia'),
      prioritario: check('prioritario')
    };
    const salarios = fabParseSalarios(field('salarios'));
    if (salarios.length) period.salarios = salarios;
    return period;
  });
  if (!periods.length) throw new Error('Informe pelo menos um período de contribuição.');
  const payload = {
    dataApuracao,
    dadosSegurado: {
      nascimento,
      sexo: value('sexo') || 'F',
      tipo: value('tipo') || 'COMUM'
    },
    opcoesContagem: { especie: Number(value('especie')) || 42 },
    periodosContribuicao: periods
  };
  if (value('der')) payload.der = value('der');
  if (form.dataset.fabMode === 'rmi') {
    payload.fundamento = value('fundamento') || 'EC_103_ART_20';
    payload.marcoTemporal = value('marcoTemporal') || 'NA_DIB';
    if (checked('aplicarIrsmFev94')) payload.aplicarIrsmFev94 = true;
  }
  return payload;
}

function fabSetBusy(form, busy) {
  if (!form) return;
  const buttons = $$('button', form);
  buttons.forEach((button) => { button.disabled = busy; });
  form.setAttribute('aria-busy', busy ? 'true' : 'false');
}

async function fabRun(prefix, mode) {
  const form = document.getElementById('fab-form-' + prefix);
  const output = document.getElementById('fab-result-' + prefix);
  if (!form || !output) return;
  let payload;
  try {
    payload = fabReadForm(prefix);
  } catch (error) {
    output.innerHTML = fabErrorHtml({ message: error.message, payload: null });
    return;
  }
  fabSetBusy(form, true);
  output.innerHTML = '<div class="notice">' + ico('clock', 16) + '<p><strong>Consultando a Fábrica.</strong> Cálculos longos podem levar alguns segundos; o servidor local aguarda até 180 s.</p></div>';
  try {
    const result = mode === 'rmi'
      ? await FabricaApi.rmiPorFundamento(payload)
      : await FabricaApi.tempo(payload);
    output.innerHTML = mode === 'rmi' ? fabRenderRmi(result) : fabRenderTempo(result);
  } catch (error) {
    output.innerHTML = fabErrorHtml(error);
  } finally {
    fabSetBusy(form, false);
  }
}

function fabAddPeriod(prefix) {
  const wrap = document.querySelector('[data-fab-periods="' + prefix + '"]');
  if (!wrap) return;
  const index = $$('[data-fab-period]', wrap).length;
  wrap.insertAdjacentHTML('beforeend', fabPeriodRow(prefix, index, {}));
  fabRenumber(prefix);
  const last = wrap.querySelector('[data-fab-period]:last-of-type input');
  if (last) last.focus();
}

function fabRenumber(prefix) {
  const wrap = document.querySelector('[data-fab-periods="' + prefix + '"]');
  if (!wrap) return;
  $$('[data-fab-period]', wrap).forEach((row, index) => {
    const number = row.querySelector('[data-fab-period-number]');
    if (number) number.textContent = String(index + 1);
  });
}

function fabRemovePeriod(prefix, button) {
  const wrap = document.querySelector('[data-fab-periods="' + prefix + '"]');
  if (!wrap) return;
  const rows = $$('[data-fab-period]', wrap);
  if (rows.length <= 1) {
    toast('Mantenha pelo menos um período.', 'error');
    return;
  }
  const row = button.closest('[data-fab-period]');
  if (row) row.remove();
  fabRenumber(prefix);
}

function fabFillForm(prefix, data) {
  const form = document.getElementById('fab-form-' + prefix);
  if (!form || !data) return false;
  const set = (name, value) => {
    const field = form.querySelector('[data-field="' + name + '"]');
    if (field && value != null && value !== '') field.value = value;
  };
  if (data.dataApuracao) set('dataApuracao', data.dataApuracao);
  if (data.der) set('der', data.der);
  if (data.dadosSegurado) {
    set('nascimento', data.dadosSegurado.nascimento);
    set('sexo', data.dadosSegurado.sexo);
    set('tipo', data.dadosSegurado.tipo);
  }
  if (data.opcoesContagem && data.opcoesContagem.especie) set('especie', data.opcoesContagem.especie);
  if (data.fundamento) set('fundamento', data.fundamento);
  if (data.marcoTemporal) set('marcoTemporal', data.marcoTemporal);
  if (Array.isArray(data.periodosContribuicao) && data.periodosContribuicao.length) {
    const wrap = form.querySelector('[data-fab-periods]');
    if (wrap) wrap.innerHTML = fabPeriodsHtml(prefix, data.periodosContribuicao);
    fabRenumber(prefix);
  }
  return true;
}

function fabUseCase(prefix) {
  const data = fabDataFromCase();
  if (!fabFillForm(prefix, data)) {
    toast('Não foi possível preencher o formulário.', 'error');
    return;
  }
  toast('Formulário preenchido com o caso atual. Revise os dados antes de calcular.');
}

function fabConnectionNotice(message, kind) {
  const notice = document.getElementById('fabConnNotice');
  if (!notice) return;
  notice.hidden = false;
  notice.className = 'notice' + (kind === 'bad' ? ' notice--bad' : '');
  notice.innerHTML = ico(kind === 'bad' ? 'alert' : 'lock', 16) + '<div><strong>' + esc(message) + '</strong><p class="form-note">A chave da Fábrica fica no servidor local (<span class="mono">.env</span>). O navegador só conversa com o proxy.</p></div>';
}

async function fabCheckHealth() {
  const badge = document.getElementById('fabStatus');
  if (badge) {
    badge.textContent = 'Verificando';
    badge.className = 'state state--quiet';
  }
  try {
    const health = await FabricaApi.health();
    FAB.health = health;
    if (badge) {
      badge.textContent = health.configured ? 'Conectado' : 'Sem chave';
      badge.className = 'state ' + (health.configured ? 'state--ok' : 'state--warn');
    }
    if (!health.configured) fabConnectionNotice('O proxy respondeu, mas a chave não está configurada no servidor.', 'bad');
    else {
      const notice = document.getElementById('fabConnNotice');
      if (notice) notice.hidden = true;
    }
    return health;
  } catch (error) {
    FAB.health = null;
    if (badge) {
      badge.textContent = 'Offline';
      badge.className = 'state state--bad';
    }
    fabConnectionNotice('Servidor local offline. Rode node server/index.js e abra esta página pelo endereço local.', 'bad');
    return null;
  }
}

async function fabLoadCatalog() {
  if (!FAB.health || !FAB.health.configured) return;
  try {
    const results = await Promise.all([FabricaApi.fundamentos(), FabricaApi.marcosTemporais()]);
    FAB.fundamentos = results[0];
    FAB.marcos = results[1];
    const fund = document.getElementById('fab-rmi-fundamento');
    const marco = document.getElementById('fab-rmi-marco');
    if (fund && FAB.fundamentos && Array.isArray(FAB.fundamentos.result)) {
      const current = fund.value;
      fund.innerHTML = fabOptions(FAB.fundamentos.result.map((item) => [item.valor, item.rotulo]), current || 'EC_103_ART_20');
    }
    if (marco && FAB.marcos && Array.isArray(FAB.marcos.result)) {
      const current = marco.value;
      marco.innerHTML = fabOptions(FAB.marcos.result.map((item) => [item.valor, item.rotulo]), current || 'NA_DIB');
    }
  } catch (error) {
    if (error && error.status === 0) return;
    const output = document.getElementById('fab-rmi-result');
    if (output) output.innerHTML = fabErrorHtml(error);
  }
}

function fabSetTab(tab) {
  FAB.tab = tab;
  $$('[data-fab-tab]').forEach((button) => {
    const on = button.dataset.fabTab === tab;
    button.setAttribute('aria-selected', on ? 'true' : 'false');
    button.tabIndex = on ? 0 : -1;
  });
  $$('[data-fab-panel]').forEach((panel) => {
    panel.hidden = panel.dataset.fabPanel !== tab;
  });
  if (tab === 'referencias') fabRefCjfLoad();
}

function fabTabsHtml() {
  return '<div class="tabs fab-tabs" role="tablist" aria-label="Serviços da Fábrica de Cálculos">' +
    FABRICA_TABS.map((tab) => '<button type="button" class="tab" role="tab" id="fab-tab-' + tab.id + '" data-fab-tab="' + tab.id + '" aria-selected="' + (FAB.tab === tab.id ? 'true' : 'false') + '" aria-controls="fab-panel-' + tab.id + '" tabindex="' + (FAB.tab === tab.id ? '0' : '-1') + '">' + esc(tab.label) + '</button>').join('') +
    '</div>';
}

function fabDocsPanel() {
  return '<section class="folio"><div class="folio__head"><div><div class="folio__title">Ler documento</div><div class="folio__sub">CNIS, RDCTC, Carta de Concessão, PAP ou PrevJud. O arquivo vai ao proxy local e volta como JSON estruturado.</div></div><span class="state state--quiet">POST /documentos/*/ler</span></div><div class="folio__body">' +
    '<form id="fab-doc-form" novalidate><div class="form-grid">' +
    '<div class="field"><label for="fab-doc-tipo">Tipo</label><select class="input" id="fab-doc-tipo" data-field="tipo"><option value="cnis">CNIS</option><option value="rdctc">RDCTC</option><option value="carta">Carta de Concessão</option><option value="pap">PAP</option><option value="prevjud">PrevJud</option></select></div>' +
    '<div class="field"><label for="fab-doc-file">Arquivo</label><input class="input" id="fab-doc-file" type="file" accept=".pdf,application/pdf"></div>' +
    '</div><div class="form-actions"><button type="submit" class="btn btn--primary">Ler documento</button></div><p class="form-note">PDFs de cliente não são gravados pelo proxy; o arquivo é repassado em memória. Use dados sintéticos nesta demonstração.</p></form>' +
    '<div id="fab-doc-result" class="fab-result" aria-live="polite"></div>' +
    '<div class="hr"></div>' +
    '<div class="folio__title">Arquivo .calc</div><p class="form-note">Abre ou calcula um arquivo .calc dos serviços de tempo, atrasados ou RMI. O proxy envia o multipart no campo <span class="mono">file</span>.</p>' +
    '<form id="fab-calc-form" novalidate><div class="form-grid">' +
    '<div class="field"><label for="fab-calc-tipo">Operação</label><select class="input" id="fab-calc-tipo" data-field="tipo">' +
    '<option value="tempo/calc/ler">Tempo · ler</option>' +
    '<option value="tempo/calc/calcular">Tempo · calcular</option>' +
    '<option value="tempo/calc/calcular-por-grupos">Tempo · calcular por grupos</option>' +
    '<option value="atrasados/calc/ler">Atrasados · ler</option>' +
    '<option value="atrasados/calc/calcular">Atrasados · calcular</option>' +
    '<option value="rmi/calc/ler">RMI · ler</option>' +
    '<option value="rmi/calc/ler-desvinculada">RMI · ler desvinculada</option>' +
    '<option value="rmi/calc/calcular">RMI · calcular</option>' +
    '<option value="rmi/calc/calcular-desvinculada">RMI · calcular desvinculada</option>' +
    '</select></div>' +
    '<div class="field"><label for="fab-calc-file">Arquivo .calc</label><input class="input" id="fab-calc-file" type="file" accept=".calc,application/json"></div>' +
    '</div><div class="form-actions"><button type="submit" class="btn btn--quiet">Abrir / calcular .calc</button></div></form>' +
    '<div id="fab-calc-result" class="fab-result" aria-live="polite"></div></div></section>';
}

function fabIndicesPanel() {
  return '<section class="folio"><div class="folio__head"><div><div class="folio__title">Índices, limites e tabelas</div><div class="folio__sub">Consulta os mesmos índices usados pela API de cálculo. Cache do proxy não é persistente.</div></div><span class="state state--quiet">GET /indices/api/v1/*</span></div><div class="folio__body">' +
    '<div class="fab-index-form"><div class="form-grid"><div class="field"><label for="fab-index-inicio">Competência inicial</label><input class="input" id="fab-index-inicio" type="month" value="' + esc(fabCompetencia(-12)) + '"></div><div class="field"><label for="fab-index-fim">Competência final</label><input class="input" id="fab-index-fim" type="month" value="' + esc(fabCompetencia(0)) + '"></div><div class="field"><label for="fab-index-indice">Índice (opcional)</label><input class="input" id="fab-index-indice" type="text" placeholder="INPC, IPCA, SELIC..."></div></div>' +
    '<div class="form-actions" style="justify-content:flex-start">' +
    '<button type="button" class="btn btn--quiet" data-fab-index="limites">Limites</button>' +
    '<button type="button" class="btn btn--quiet" data-fab-index="tabela">Tabela</button>' +
    '<button type="button" class="btn btn--quiet" data-fab-index="reajuste">Reajuste</button>' +
    '<button type="button" class="btn btn--quiet" data-fab-index="moedas">Moedas</button>' +
    '<button type="button" class="btn btn--quiet" data-fab-index="tabua-mortalidade">Tábua de mortalidade</button>' +
    '<button type="button" class="btn btn--quiet" data-fab-index="adiantamento">Adiantamento</button>' +
    '<button type="button" class="btn btn--quiet" data-fab-index="limite-baixa-renda-reclusao">Baixa renda / reclusão</button>' +
    '</div></div><div id="fab-index-result" class="fab-result" aria-live="polite">' + (FAB.indexResult ? fabRenderGeneric(FAB.indexResult) : '') + '</div></div></section>';
}

function fabCompetencia(offset) {
  const date = new Date();
  date.setDate(1);
  date.setMonth(date.getMonth() + offset);
  return date.getFullYear() + '-' + String(date.getMonth() + 1).padStart(2, '0');
}

async function fabLoadIndex(name, button) {
  const output = document.getElementById('fab-index-result');
  if (!output) return;
  if (button) button.disabled = true;
  output.innerHTML = '<p class="muted">Consultando ' + esc(name) + '…</p>';
  try {
    const params = {};
    if (name === 'tabela' || name === 'reajuste') {
      const inicio = document.getElementById('fab-index-inicio');
      const fim = document.getElementById('fab-index-fim');
      const indice = document.getElementById('fab-index-indice');
      if (inicio && inicio.value) params.competenciaInicial = inicio.value;
      if (fim && fim.value) params.competenciaFinal = fim.value;
      if (name === 'tabela' && indice && indice.value.trim()) params.indice = indice.value.trim();
    }
    const result = await FabricaApi.indices(name, params);
    FAB.indexResult = result;
    output.innerHTML = fabRenderGeneric(result);
  } catch (error) {
    output.innerHTML = fabErrorHtml(error);
  } finally {
    if (button) button.disabled = false;
  }
}

function fabAtrasadosPanel() {
  const model = {
    dadosProcesso: { numero: '1234567-88.1900.0.00.0000', protocolo: '2026-01-01' },
    opcoesCalculo: { dataAtualizacao: '2026-01-01', consectariosPrincipal: {}, consectariosAcessorios: {}, criterioDeducaoAlcada: '', deduzirExcedenteAlcada: false },
    beneficiosDevidos: [],
    beneficiosPagos: [],
    periodosBeneficioVantajoso: [],
    descontos: [],
    acrescimos: [],
    outroDebitos: [],
    honorarios: {},
    multa: {}
  };
  return '<section class="folio"><div class="folio__head"><div><div class="folio__title">Liquidação de atrasados</div><div class="folio__sub">O schema de atrasados é extenso (processo, benefícios devidos e pagos, consectários, descontos, honorários e multa). Edite o JSON e envie; a API devolve o caminho exato do campo inválido.</div></div><span class="state state--quiet">POST /calculos-atrasados</span></div><div class="folio__body">' +
    '<form id="fab-atrasados-form"><div class="field"><label for="fab-atrasados-body">Corpo JSON</label><textarea class="textarea fab-code" id="fab-atrasados-body" spellcheck="false" rows="18">' + esc(JSON.stringify(model, null, 2)) + '</textarea><span class="field__hint">Modelo inicial incompleto de propósito: preencha os consectários e os benefícios. Nenhum valor é presumido.</span></div>' +
    '<div class="form-actions"><button type="button" class="btn btn--ghost" data-fab-atrasados-model>Recarregar modelo</button><button type="submit" class="btn btn--primary">Calcular atrasados</button></div></form>' +
    '<div id="fab-atrasados-result" class="fab-result" aria-live="polite"></div></div></section>';
}

function fabAtrasadosModel() {
  const body = document.getElementById('fab-atrasados-body');
  if (!body) return;
  body.value = JSON.stringify({
    dadosProcesso: { numero: '1234567-88.1900.0.00.0000', protocolo: '2026-01-01' },
    opcoesCalculo: { dataAtualizacao: '2026-01-01', consectariosPrincipal: {}, consectariosAcessorios: {}, criterioDeducaoAlcada: '', deduzirExcedenteAlcada: false },
    beneficiosDevidos: [],
    beneficiosPagos: [],
    periodosBeneficioVantajoso: [],
    descontos: [],
    acrescimos: [],
    outroDebitos: [],
    honorarios: {},
    multa: {}
  }, null, 2);
}

async function fabAtrasadosSend(event) {
  event.preventDefault();
  const body = document.getElementById('fab-atrasados-body');
  const output = document.getElementById('fab-atrasados-result');
  if (!body || !output) return;
  let payload;
  try {
    payload = JSON.parse(body.value || '{}');
  } catch (error) {
    output.innerHTML = fabErrorHtml({ message: 'JSON inválido: ' + error.message, payload: null });
    return;
  }
  output.innerHTML = '<div class="notice">' + ico('clock', 16) + '<p><strong>Liquidando atrasados.</strong> O servidor local aguarda até 180 s.</p></div>';
  try {
    const result = await FabricaApi.atrasados(payload);
    output.innerHTML = fabRenderGeneric(result);
  } catch (error) {
    output.innerHTML = fabErrorHtml(error);
  }
}

function fabRefTable(headers, rows, formatter) {
  const head = (headers || []).map((header) => '<th>' + esc(header) + '</th>').join('');
  const body = (rows || []).slice(0, 200).map((row, index) => '<tr>' + (Array.isArray(row) ? row : []).map((cell, cellIndex) => '<td' + (cellIndex === 0 ? ' class="mono"' : '') + '>' + esc(formatter ? formatter(cell, cellIndex) : cell) + '</td>').join('') + '</tr>').join('');
  return '<div class="table-wrap fab-ref-table" tabindex="0" aria-label="Tabela de referência"><table class="table"><thead><tr>' + head + '</tr></thead><tbody>' + body + '</tbody></table></div>';
}

function fabRefMeta(payload) {
  return '<div class="fab-ref-meta"><span>' + esc(payload.source || 'fonte pública') + '</span><span>' + (payload.cached ? 'cache hit' : 'cache miss') + '</span><span>' + (payload.result && payload.result.series ? payload.result.series.length + ' ponto(s)' : '') + '</span></div>';
}

function fabRefBcbHtml(payload) {
  const serie = payload.result && payload.result.serie;
  const rows = payload.result && Array.isArray(payload.result.series) ? payload.result.series : [];
  if (!rows.length) return fabRefMeta(payload) + '<p class="muted">Sem dados no período.</p>';
  const format = (value) => serie === '1619' ? brl(Number(value)) : Number(value).toLocaleString('pt-BR', { minimumFractionDigits: 2, maximumFractionDigits: 4 }) + '%';
  return fabRefMeta(payload) + fabRefTable(['Data', 'Valor'], rows.map((row) => [row.data, format(row.valor)]));
}

function fabRefIbgeHtml(payload) {
  const rows = payload.result && Array.isArray(payload.result.series) ? payload.result.series : [];
  if (!rows.length) return fabRefMeta(payload) + '<p class="muted">Sem dados no período.</p>';
  return fabRefMeta(payload) + fabRefTable(['Competência', 'Variação'], rows.map((row) => [row.periodo, Number(row.valor).toLocaleString('pt-BR', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + '%']));
}

function fabRefCjfHtml(payload) {
  const result = payload.result || {};
  const rows = Array.isArray(result.rows) ? result.rows : [];
  const headers = Array.isArray(result.headers) ? result.headers : [];
  const meta = '<div class="fab-ref-meta"><span>' + esc(payload.source || 'CJF SICOM') + '</span><span>' + (payload.cached ? 'cache do servidor' : 'importado agora') + '</span><span>' + rows.length + ' linha(s)</span></div>';
  if (!headers.length) return meta + '<p class="muted">Nenhuma tabela CJF importada nesta execução. Exporte o CSV no SICOM e envie abaixo.</p>';
  return meta + fabRefTable(headers, rows);
}

function fabReferenciasPanel() {
  return '<section class="folio"><div class="folio__head"><div><div class="folio__title">Referências nacionais</div><div class="folio__sub">Séries públicas para conferência, atualização e leitura de contexto. Não substituem a Fábrica no cálculo judicial.</div></div><span class="state state--quiet">BCB · IBGE · CJF</span></div><div class="folio__body">' +
    '<div class="fab-ref-grid">' +
    '<div class="fab-ref-card"><div class="folio__title">Banco Central — SGS</div><p class="form-note">Salário mínimo, INPC, IPCA e IGP-M. Cache do proxy por 6 h; o SGS limita consultas longas a 10 anos.</p>' +
    '<div class="form-grid"><div class="field"><label for="fab-ref-bcb-serie">Série</label><select class="input" id="fab-ref-bcb-serie"><option value="1619">1619 · Salário mínimo</option><option value="188">188 · INPC</option><option value="433">433 · IPCA</option><option value="189">189 · IGP-M</option></select></div><div class="field"><label for="fab-ref-bcb-ultimos">Últimos</label><input class="input" id="fab-ref-bcb-ultimos" type="number" min="1" max="120" value="12"></div></div>' +
    '<div class="form-actions" style="justify-content:flex-start"><button type="button" class="btn btn--quiet" data-fab-ref-bcb>Consultar BCB</button></div><div id="fab-ref-bcb-result" class="fab-result" aria-live="polite">' + (FAB.refBcb ? fabRefBcbHtml(FAB.refBcb) : '') + '</div>' +
    '<p class="form-note"><a href="https://api.bcb.gov.br" target="_blank" rel="noopener">api.bcb.gov.br</a> · fonte oficial, sem chave.</p></div>' +
    '<div class="fab-ref-card"><div class="folio__title">IBGE — SIDRA</div><p class="form-note">IPCA e INPC mensais do Brasil. O proxy normaliza a série e guarda em cache por 6 h.</p>' +
    '<div class="form-grid"><div class="field"><label for="fab-ref-ibge-serie">Série</label><select class="input" id="fab-ref-ibge-serie"><option value="ipca">IPCA</option><option value="inpc">INPC</option></select></div><div class="field"><label for="fab-ref-ibge-meses">Meses</label><input class="input" id="fab-ref-ibge-meses" type="number" min="1" max="120" value="12"></div></div>' +
    '<div class="form-actions" style="justify-content:flex-start"><button type="button" class="btn btn--quiet" data-fab-ref-ibge>Consultar IBGE</button></div><div id="fab-ref-ibge-result" class="fab-result" aria-live="polite">' + (FAB.refIbge ? fabRefIbgeHtml(FAB.refIbge) : '') + '</div>' +
    '<p class="form-note"><a href="https://servicodados.ibge.gov.br/api/docs/agregados?versao=3" target="_blank" rel="noopener">servicodados.ibge.gov.br</a> · fonte oficial, sem chave.</p></div>' +
    '<div class="fab-ref-card fab-ref-card--wide"><div class="folio__title">CJF — SICOM (CSV)</div><p class="form-note">Importe a tabela de correção monetária exportada do SICOM. O arquivo é lido pelo proxy e fica apenas na memória do servidor; não fazemos scraping da calculadora.</p>' +
    '<div class="form-grid"><div class="field"><label for="fab-ref-cjf-file">Arquivo CSV</label><input class="input" id="fab-ref-cjf-file" type="file" accept=".csv,.txt,text/csv"></div><div class="field" style="align-content:end"><button type="button" class="btn btn--quiet" data-fab-ref-cjf-import>Importar CSV</button></div></div>' +
    '<div id="fab-ref-cjf-result" class="fab-result" aria-live="polite">' + (FAB.refCjf ? fabRefCjfHtml(FAB.refCjf) : '') + '</div>' +
    '<p class="form-note"><a href="https://sicom.cjf.jus.br/tabelaCorMor.php" target="_blank" rel="noopener">SICOM/CJF</a> · o CJF não publica API REST documentada; CSV é o caminho estável.</p></div>' +
    '</div>' +
    '<div class="notice">' + ico('shield', 16) + '<p><strong>Conecta/INSS fora por enquanto.</strong> A API de benefícios exige credenciamento, chave, whitelist de IP e base legal para dados pessoais. Sem isso, não integramos.</p></div>' +
    '<div class="notice">' + ico('lock', 16) + '<p><strong>DataJud/Atrium é para processo.</strong> Metadados e publicações ficam no Atrium MCP; cálculo e referências ficam na Fábrica e nesta aba. Nenhum CPF ou documento é enviado a estas fontes.</p></div>' +
    '</div></section>';
}

async function fabRefBcb() {
  const serie = document.getElementById('fab-ref-bcb-serie');
  const ultimos = document.getElementById('fab-ref-bcb-ultimos');
  const output = document.getElementById('fab-ref-bcb-result');
  if (!serie || !ultimos || !output) return;
  output.innerHTML = '<p class="muted">Consultando o Banco Central…</p>';
  try {
    const result = await FabricaApi.referencias.bcb(serie.value, ultimos.value);
    FAB.refBcb = result;
    output.innerHTML = fabRefBcbHtml(result);
  } catch (error) {
    output.innerHTML = fabErrorHtml(error);
  }
}

async function fabRefIbge() {
  const serie = document.getElementById('fab-ref-ibge-serie');
  const meses = document.getElementById('fab-ref-ibge-meses');
  const output = document.getElementById('fab-ref-ibge-result');
  if (!serie || !meses || !output) return;
  output.innerHTML = '<p class="muted">Consultando o IBGE…</p>';
  try {
    const result = await FabricaApi.referencias.ibge(serie.value, meses.value);
    FAB.refIbge = result;
    output.innerHTML = fabRefIbgeHtml(result);
  } catch (error) {
    output.innerHTML = fabErrorHtml(error);
  }
}

async function fabRefCjfImport() {
  const input = document.getElementById('fab-ref-cjf-file');
  const output = document.getElementById('fab-ref-cjf-result');
  if (!input || !output) return;
  const file = input.files && input.files[0];
  if (!file) {
    output.innerHTML = fabErrorHtml({ message: 'Selecione um CSV exportado do SICOM.', payload: null });
    return;
  }
  output.innerHTML = '<p class="muted">Importando a tabela CJF…</p>';
  try {
    const result = await FabricaApi.referencias.cjfImport(file);
    FAB.refCjf = result;
    FAB.refCjfLoaded = true;
    output.innerHTML = fabRefCjfHtml(result);
    toast('Tabela CJF importada no cache do servidor.');
  } catch (error) {
    output.innerHTML = fabErrorHtml(error);
  }
}

async function fabRefCjfLoad() {
  if (FAB.refCjfLoaded) return;
  FAB.refCjfLoaded = true;
  try {
    const result = await FabricaApi.referencias.cjfCached();
    if (!result || !result.result || !result.result.rows || !result.result.rows.length) return;
    FAB.refCjf = result;
    const output = document.getElementById('fab-ref-cjf-result');
    if (output) output.innerHTML = fabRefCjfHtml(result);
  } catch (error) {
    FAB.refCjfLoaded = false;
  }
}

function fabConsolePanel() {
  return '<section class="folio"><div class="folio__head"><div><div class="folio__title">Console da API</div><div class="folio__sub">Acesso direto às rotas do proxy. Serve para os serviços que ainda não têm formulário guiado e para reproduzir um payload exato.</div></div><span class="state state--quiet">JSON</span></div><div class="folio__body">' +
    '<form id="fab-console-form"><div class="form-grid"><div class="field"><label for="fab-console-endpoint">Endpoint local</label><select class="input" id="fab-console-endpoint" data-field="endpoint">' +
    '<option value="/api/fabrica/tempo">POST /api/fabrica/tempo</option>' +
    '<option value="/api/fabrica/tempo/por-grupos">POST /api/fabrica/tempo/por-grupos</option>' +
    '<option value="/api/fabrica/tempo/prevjud">POST /api/fabrica/tempo/prevjud</option>' +
    '<option value="/api/fabrica/tempo/prevjud/por-grupos">POST /api/fabrica/tempo/prevjud/por-grupos</option>' +
    '<option value="/api/fabrica/rmi">POST /api/fabrica/rmi</option>' +
    '<option value="/api/fabrica/rmi/por-fundamento">POST /api/fabrica/rmi/por-fundamento</option>' +
    '<option value="/api/fabrica/rmi/desvinculada">POST /api/fabrica/rmi/desvinculada</option>' +
    '<option value="/api/fabrica/rmi/async">POST /api/fabrica/rmi/async</option>' +
    '<option value="/api/fabrica/rmi/stream">POST /api/fabrica/rmi/stream</option>' +
    '<option value="/api/fabrica/atrasados">POST /api/fabrica/atrasados</option>' +
    '<option value="/api/fabrica/mcp">POST /api/fabrica/mcp</option>' +
    '<option value="/api/fabrica/fundamentos">GET /api/fabrica/fundamentos</option>' +
    '<option value="/api/fabrica/marcos-temporais">GET /api/fabrica/marcos-temporais</option>' +
    '<option value="/api/fabrica/documentos-legais">GET /api/fabrica/documentos-legais</option>' +
    '<option value="/api/fabrica/indices/limites">GET /api/fabrica/indices/limites</option>' +
    '</select></div><div class="field"><label for="fab-console-query">Query (opcional)</label><input class="input" id="fab-console-query" type="text" placeholder="demonstrativo=true"></div></div>' +
    '<div class="field" style="margin-top:13px"><label for="fab-console-body">Corpo JSON</label><textarea class="textarea fab-code" id="fab-console-body" spellcheck="false" rows="12">{}</textarea><span class="field__hint">Para GET, o corpo é ignorado. Para MCP, use o JSON-RPC; a chave continua no servidor.</span></div>' +
    '<div class="form-actions"><button type="button" class="btn btn--ghost" data-fab-console-model>Carregar modelo</button><button type="submit" class="btn btn--primary">Enviar</button></div></form>' +
    '<div id="fab-console-result" class="fab-result" aria-live="polite">' + (FAB.consoleResult ? fabRenderGeneric(FAB.consoleResult) : '') + '</div></div></section>';
}

function fabConsoleModel() {
  const endpoint = document.getElementById('fab-console-endpoint');
  const body = document.getElementById('fab-console-body');
  if (!endpoint || !body) return;
  if (endpoint.value.indexOf('/prevjud') >= 0) {
    body.value = '{}';
  } else if (endpoint.value.indexOf('/tempo') >= 0 || endpoint.value.indexOf('/rmi') >= 0) {
    const isRmi = endpoint.value.indexOf('/rmi') >= 0;
    const data = fabDefaultData(isRmi ? 'rmi' : 'tempo');
    if (data.der === '') delete data.der;
    if (!isRmi) {
      delete data.fundamento;
      delete data.marcoTemporal;
      delete data.aplicarIrsmFev94;
    } else if (endpoint.value.indexOf('/rmi/por-fundamento') < 0) {
      delete data.fundamento;
    }
    body.value = JSON.stringify(data, null, 2);
  } else if (endpoint.value.indexOf('/atrasados') >= 0) {
    body.value = JSON.stringify({
      dadosProcesso: { numero: '1234567-88.1900.0.00.0000', protocolo: '2026-01-01' },
      opcoesCalculo: {},
      beneficiosDevidos: [],
      beneficiosPagos: [],
      periodosBeneficioVantajoso: [],
      descontos: [],
      acrescimos: [],
      outroDebitos: [],
      honorarios: {},
      multa: {}
    }, null, 2);
  } else {
    body.value = '{}';
  }
}

async function fabConsoleSend(event) {
  event.preventDefault();
  const endpoint = document.getElementById('fab-console-endpoint');
  const query = document.getElementById('fab-console-query');
  const body = document.getElementById('fab-console-body');
  const output = document.getElementById('fab-console-result');
  if (!endpoint || !body || !output) return;
  const isGet = endpoint.value.indexOf('/fundamentos') >= 0 || endpoint.value.indexOf('/marcos-temporais') >= 0 || endpoint.value.indexOf('/indices/') >= 0 || endpoint.value.indexOf('/documentos-legais') >= 0;
  let parsed = null;
  if (!isGet) {
    try {
      parsed = JSON.parse(body.value || '{}');
    } catch (error) {
      output.innerHTML = fabErrorHtml({ message: 'JSON inválido: ' + error.message, payload: null });
      return;
    }
  }
  let search = String(query && query.value || '').trim();
  if (search && search.charAt(0) !== '?') search = '?' + search;
  output.innerHTML = '<p class="muted">Enviando…</p>';
  try {
    const result = await FabricaApi.request(endpoint.value + search, { method: isGet ? 'GET' : 'POST', body: parsed, timeout: 200000 });
    FAB.consoleResult = result;
    output.innerHTML = fabRenderGeneric(result);
  } catch (error) {
    output.innerHTML = fabErrorHtml(error);
  }
}

function fabWire() {
  if (fabWired) return;
  fabWired = true;
  document.addEventListener('click', (event) => {
    const tab = event.target.closest('[data-fab-tab]');
    if (tab) {
      fabSetTab(tab.dataset.fabTab);
      return;
    }
    const add = event.target.closest('[data-fab-add-period]');
    if (add) {
      fabAddPeriod(add.dataset.fabAddPeriod);
      return;
    }
    const remove = event.target.closest('[data-fab-remove-period]');
    if (remove) {
      fabRemovePeriod(remove.dataset.fabRemovePeriod, remove);
      return;
    }
    const useCase = event.target.closest('[data-fab-use-case]');
    if (useCase) {
      fabUseCase(useCase.dataset.fabUseCase);
      return;
    }
    const health = event.target.closest('[data-fab-health]');
    if (health) {
      fabCheckHealth().then((result) => { if (result && result.configured) fabLoadCatalog(); });
      return;
    }
    const saveBase = event.target.closest('[data-fab-save-base]');
    if (saveBase) {
      const input = document.getElementById('fab-base');
      FabricaApi.setBase(input ? input.value : '');
      fabCheckHealth().then((result) => { if (result && result.configured) fabLoadCatalog(); });
      toast('Endereço do proxy salvo neste navegador.');
      return;
    }
    const index = event.target.closest('[data-fab-index]');
    if (index) {
      fabLoadIndex(index.dataset.fabIndex, index);
      return;
    }
    const useDoc = event.target.closest('[data-fab-use-doc]');
    if (useDoc) {
      const result = FAB.docsResult && FAB.docsResult.result;
      const candidate = result && Array.isArray(result.periodosContribuicao) ? result : result && result.dados && Array.isArray(result.dados.periodosContribuicao) ? result.dados : null;
      if (!candidate) {
        toast('O documento não devolveu períodos de contribuição utilizáveis.', 'error');
        return;
      }
      const filled = fabFillForm('tempo', candidate) || fabFillForm('rmi', candidate);
      if (filled) {
        fabSetTab('tempo');
        toast('Períodos importados para o formulário de tempo. Revise antes de calcular.');
      } else {
        toast('Não foi possível preencher o formulário.', 'error');
      }
      return;
    }
    if (event.target.closest('[data-fab-ref-bcb]')) {
      fabRefBcb();
      return;
    }
    if (event.target.closest('[data-fab-ref-ibge]')) {
      fabRefIbge();
      return;
    }
    if (event.target.closest('[data-fab-ref-cjf-import]')) {
      fabRefCjfImport();
      return;
    }
    const model = event.target.closest('[data-fab-console-model]');
    if (model) {
      fabConsoleModel();
      return;
    }
    if (event.target.closest('[data-fab-atrasados-model]')) {
      fabAtrasadosModel();
      return;
    }
  });
  document.addEventListener('keydown', (event) => {
    const tab = event.target.closest('[data-fab-tab]');
    if (!tab) return;
    const tabs = $$('[data-fab-tab]');
    const index = tabs.indexOf(tab);
    let next = null;
    if (event.key === 'ArrowRight') next = (index + 1) % tabs.length;
    if (event.key === 'ArrowLeft') next = (index - 1 + tabs.length) % tabs.length;
    if (event.key === 'Home') next = 0;
    if (event.key === 'End') next = tabs.length - 1;
    if (next == null) return;
    event.preventDefault();
    tabs[next].focus();
    fabSetTab(tabs[next].dataset.fabTab);
  });
  document.addEventListener('submit', (event) => {
    if (event.target.id === 'fab-form-tempo') {
      event.preventDefault();
      fabRun('tempo', 'tempo');
      return;
    }
    if (event.target.id === 'fab-form-rmi') {
      event.preventDefault();
      fabRun('rmi', 'rmi');
      return;
    }
    if (event.target.id === 'fab-doc-form') {
      event.preventDefault();
      fabSubmitDoc();
      return;
    }
    if (event.target.id === 'fab-calc-form') {
      event.preventDefault();
      fabCalcSubmit();
      return;
    }
    if (event.target.id === 'fab-console-form') {
      fabConsoleSend(event);
      return;
    }
    if (event.target.id === 'fab-atrasados-form') {
      fabAtrasadosSend(event);
    }
  });
}

async function fabCalcSubmit() {
  const type = document.getElementById('fab-calc-tipo');
  const input = document.getElementById('fab-calc-file');
  const output = document.getElementById('fab-calc-result');
  if (!type || !input || !output) return;
  const file = input.files && input.files[0];
  if (!file) {
    output.innerHTML = fabErrorHtml({ message: 'Selecione um arquivo .calc.', payload: null });
    return;
  }
  const form = document.getElementById('fab-calc-form');
  fabSetBusy(form, true);
  output.innerHTML = '<p class="muted">Enviando o arquivo .calc…</p>';
  try {
    const formData = new FormData();
    formData.append('file', file, file.name || 'calculo.calc');
    const result = await FabricaApi.request('/api/fabrica/' + type.value, { method: 'POST', body: formData, timeout: 200000 });
    FAB.docsResult = result;
    const candidate = result && result.result && Array.isArray(result.result.periodosContribuicao);
    output.innerHTML = (candidate ? '<div class="form-actions" style="justify-content:flex-start"><button type="button" class="btn btn--primary" data-fab-use-doc>Usar no cálculo de tempo</button></div>' : '') + fabRenderGeneric(result);
  } catch (error) {
    output.innerHTML = fabErrorHtml(error);
  } finally {
    fabSetBusy(form, false);
  }
}

async function fabSubmitDoc() {
  const type = document.getElementById('fab-doc-tipo');
  const input = document.getElementById('fab-doc-file');
  const output = document.getElementById('fab-doc-result');
  if (!type || !input || !output) return;
  const file = input.files && input.files[0];
  if (!file) {
    output.innerHTML = fabErrorHtml({ message: 'Selecione um PDF.', payload: null });
    return;
  }
  const form = document.getElementById('fab-doc-form');
  fabSetBusy(form, true);
  output.innerHTML = '<p class="muted">Lendo o documento na Fábrica…</p>';
  try {
    const result = await FabricaApi.documento(type.value, file);
    FAB.docsResult = result;
    const candidate = result && result.result && (Array.isArray(result.result.periodosContribuicao) || (result.result.dados && Array.isArray(result.result.dados.periodosContribuicao)));
    output.innerHTML = '<div class="notice">' + ico('check', 16) + '<p><strong>Documento lido.</strong> A resposta estruturada está abaixo.</p></div>' +
      (candidate ? '<div class="form-actions" style="justify-content:flex-start"><button type="button" class="btn btn--primary" data-fab-use-doc>Usar no cálculo de tempo</button></div>' : '') +
      fabRenderGeneric(result);
  } catch (error) {
    output.innerHTML = fabErrorHtml(error);
  } finally {
    fabSetBusy(form, false);
  }
}

function viewFabrica() {
  return pageHead(
    'Fábrica de Cálculos',
    'Tempo, RMI, atrasados, documentos, índices e referências oficiais. O navegador fala com o proxy local; a chave fica no servidor.',
    '<span class="state state--quiet" id="fabStatus">Verificando</span><button type="button" class="btn btn--quiet" data-fab-health>Testar conexão</button>'
  ) +
    '<div class="notice" id="fabConnNotice" hidden></div>' +
    '<details class="fab-config"><summary>Conexão local</summary><div class="form-grid"><div class="field"><label for="fab-base">Endereço do proxy</label><input class="input" id="fab-base" type="url" value="' + esc(FabricaApi.base || '') + '" placeholder="http://127.0.0.1:8787"><span class="field__hint">Em branco quando a página é servida pelo próprio proxy.</span></div><div class="field" style="align-content:end"><button type="button" class="btn btn--quiet" data-fab-save-base>Salvar endereço</button></div></div></details>' +
    fabTabsHtml() +
    '<div class="fab-panels">' +
    '<div data-fab-panel="tempo" id="fab-panel-tempo" role="tabpanel" aria-labelledby="fab-tab-tempo"' + (FAB.tab === 'tempo' ? '' : ' hidden') + '>' + fabCalcPanel('tempo', 'tempo') + '</div>' +
    '<div data-fab-panel="rmi" id="fab-panel-rmi" role="tabpanel" aria-labelledby="fab-tab-rmi"' + (FAB.tab === 'rmi' ? '' : ' hidden') + '>' + fabCalcPanel('rmi', 'rmi') + '</div>' +
    '<div data-fab-panel="atrasados" id="fab-panel-atrasados" role="tabpanel" aria-labelledby="fab-tab-atrasados"' + (FAB.tab === 'atrasados' ? '' : ' hidden') + '>' + fabAtrasadosPanel() + '</div>' +
    '<div data-fab-panel="documentos" id="fab-panel-documentos" role="tabpanel" aria-labelledby="fab-tab-documentos"' + (FAB.tab === 'documentos' ? '' : ' hidden') + '>' + fabDocsPanel() + '</div>' +
    '<div data-fab-panel="indices" id="fab-panel-indices" role="tabpanel" aria-labelledby="fab-tab-indices"' + (FAB.tab === 'indices' ? '' : ' hidden') + '>' + fabIndicesPanel() + '</div>' +
    '<div data-fab-panel="referencias" id="fab-panel-referencias" role="tabpanel" aria-labelledby="fab-tab-referencias"' + (FAB.tab === 'referencias' ? '' : ' hidden') + '>' + fabReferenciasPanel() + '</div>' +
    '<div data-fab-panel="console" id="fab-panel-console" role="tabpanel" aria-labelledby="fab-tab-console"' + (FAB.tab === 'console' ? '' : ' hidden') + '>' + fabConsolePanel() + '</div>' +
    '</div>' +
    '<div class="notice">' + ico('lock', 16) + '<p><strong>A chave não entra no navegador.</strong> O proxy local guarda a chave no <span class="mono">.env</span> e repassa o cabeçalho <span class="mono">x-api-key</span>. Nenhum dado é gravado pelo proxy.</p></div>';
}

async function initFabricaPage() {
  fabWire();
  fabSetTab(FAB.tab);
  const health = await fabCheckHealth();
  if (health && health.configured) await fabLoadCatalog();
}
