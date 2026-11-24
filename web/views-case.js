function periodMonths(p) {
  const a = p.start.split('-').map(Number);
  const b = p.end.split('-').map(Number);
  return (b[0] - a[0]) * 12 + (b[1] - a[1]) + 1;
}

function yearFloat(ym) {
  const p = ym.split('-').map(Number);
  return p[0] + (p[1] - 1) / 12;
}

function pixelsFor(c) {
  return PERIODS[c.id] || [{ employer: 'Vínculo principal', kind: 'Empregado', start: c.start, end: c.end, contribs: c.contribs, status: 'conferido', source: 'CNIS', cnpj: '—' }];
}

function timeline(c) {
  const list = pixelsFor(c);
  const min = 1982;
  const max = 2028;
  const span = max - min;
  const pos = (ym) => ((yearFloat(ym) - min) / span) * 100;
  let marks = '';
  list.forEach((p, i) => {
    const start = pos(p.start);
    const end = pos(p.end) + (100 / span / 12);
    const width = Math.max(0.35, end - start);
    const label = esc(p.employer) + ', ' + p.start + ' a ' + p.end + ', ' + p.contribs + ' contribuições';
    marks += '<button type="button" class="tl__period" style="left:' + start.toFixed(3) + '%;width:' + width.toFixed(3) + '%" data-kind="' + kindTone(p.kind) + '" data-state="' + periodTone(p.status) + '" data-period="' + i + '" aria-label="' + label + '"></button>';
    if (i > 0) {
      const prev = list[i - 1];
      const gs = pos(prev.end) + (100 / span / 12);
      const ge = start;
      if (ge - gs > 0.05) marks += '<span class="tl__gap" style="left:' + gs.toFixed(3) + '%;width:' + (ge - gs).toFixed(3) + '%"></span>';
    }
  });
  let ticks = '';
  for (let y = 1985; y <= 2025; y += 5) {
    const x = ((y - min) / span) * 100;
    ticks += '<span class="tl__tick" style="left:' + x.toFixed(3) + '%"><span>' + y + '</span></span>';
  }
  const now = ((2026 + 9 / 12 - min) / span) * 100;
  const total = list.reduce((s, p) => s + periodMonths(p), 0);
  return '<div class="ledger"><div class="ledger__head"><div><div class="ledger__title">Tempo de contribuição</div><div class="ledger__meta">' + list.length + ' vínculos, ' + total + ' competências, lacunas visíveis</div></div><div class="ledger__total">' + months(total) + '</div></div><div class="tl-wrap"><div class="tl"><div class="tl__axis"></div>' + marks + ticks + '<span class="tl__now" style="left:' + now.toFixed(3) + '%"><span>hoje</span></span></div></div><p class="form-note tl-hint">Deslize a régua para ver todos os períodos.</p><div class="tl__readout" id="periodReadout" aria-live="polite">Foque um período para ver origem, tempo e contribuições.</div><div class="legend"><span class="legend__item"><span class="legend__sample"></span>empregado</span><span class="legend__item"><span class="legend__sample legend__sample--individual"></span>contribuinte individual</span><span class="legend__item"><span class="legend__sample legend__sample--pendente"></span>pendente</span><span class="legend__item"><span class="legend__sample legend__sample--parcial"></span>parcial</span></div></div>';
}

function caseTabs() {
  const tabs = [['overview', 'Visão geral'], ['cnis', 'CNIS e vínculos'], ['contributions', 'Contribuições'], ['premises', 'Premissas'], ['memory', 'Memória'], ['reports', 'Relatórios']];
  return '<div class="tabs" role="tablist" aria-label="Seções do cálculo">' + tabs.map((t) => '<button type="button" class="tab" role="tab" data-tab="' + t[0] + '" aria-selected="' + (state.caseTab === t[0]) + '">' + t[1] + '</button>').join('') + '</div>';
}

function pendenciesFor(c) {
  if (Array.isArray(c.pendencies) && c.pendencies.length) return c.pendencies;
  if (state.workspaceSource === 'demo') {
    if (c.id === 4) return [
      { tone: 'bad', t: 'Tabela de contribuições ausente', d: 'O CNIS recebido tem 14 páginas e para na tabela de vínculos. Sem as competências, o cálculo não avança.' },
      { tone: 'warn', t: 'DIB a confirmar', d: 'A data foi declarada no cadastro, não no documento.' }
    ];
    if (c.id === 6) return [
      { tone: 'bad', t: 'PPP do período especial pendente', d: 'Sem o PPP, o tempo especial não entra na memória. O caso permanece bloqueado.' },
      { tone: 'warn', t: 'Ruído e calor sem laudo', d: 'A exposição precisa de laudo técnico e informação de empresa.' }
    ];
    if (c.id === 2) return [
      { tone: 'warn', t: 'CTC de 1994 a 2005', d: 'A regra de pontos depende da conferência do tempo total.' },
      { tone: 'ok', t: 'Vínculos conferidos', d: 'Três vínculos com página de origem registrada.' }
    ];
  }
  const tone = c.status === 'Bloqueado' ? 'bad' : (c.status === 'Pendente' || c.status === 'Em revisão') ? 'warn' : 'ok';
  const title = c.status === 'Bloqueado' ? 'Caso bloqueado' : c.status === 'Pendente' ? 'Dados ausentes' : c.status === 'Em revisão' ? 'Revisão em aberto' : 'Conferência registrada';
  const detail = c.status === 'Bloqueado' ? 'Falta um documento essencial; o resultado não é calculado.' : c.status === 'Pendente' ? 'Complete o CNIS ou as premissas antes de recalcular.' : c.status === 'Em revisão' ? 'Confira os vínculos e as premissas antes de gerar o relatório.' : 'Vínculos e contribuições com origem declarada; revise antes de exportar.';
  return [{ tone: tone, t: title, d: detail }];
}

function resultPanel(c) {
  const blocked = c.status === 'Bloqueado' || c.status === 'Pendente';
  const rows = [
    ['RMI', c.rmi == null ? '—' : brl(c.rmi), 'gold'],
    ['Tempo', months(c.months), ''],
    ['Carência', String(c.contribs), ''],
    ['Regra', c.rule, ''],
    ['Atualizado', dateBR(c.updated), '']
  ].map((r) => '<div class="ledger-row"><dt>' + r[0] + '</dt><dd class="' + (r[2] || '') + '">' + esc(r[1]) + '</dd></div>').join('');
  const pend = pendenciesFor(c).map((p) => '<div class="pendency pendency--' + (p.tone === 'bad' ? 'bad' : p.tone === 'ok' ? 'ok' : '') + '"><span class="pendency__mark"></span><div class="pendency__copy"><strong>' + esc(p.t) + '</strong><span>' + esc(p.d) + '</span></div></div>').join('');
  return '<section class="folio"><div class="folio__head"><div class="folio__title">Resultado</div><span class="stamp ' + (blocked ? 'stamp--bad' : 'stamp--warn') + '">' + (blocked ? 'Bloqueado' : 'Experimental') + '</span></div><div class="folio__body"><dl class="ledger-list" style="margin:0">' + rows + '</dl><p class="form-note">Motor 0.10.5. Resultado experimental; não certifica direito e não substitui revisão profissional.</p></div></section>' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Pendências</div><span class="folio__sub">' + pendenciesFor(c).length + '</span></div><div class="folio__body">' + pend + '</div></section>' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Origem</div></div><div class="folio__body"><dl class="ledger-list" style="margin:0"><div class="ledger-row"><dt>CNIS</dt><dd class="mono">' + (c.cnisPages ? c.cnisPages + ' páginas' : '—') + '</dd></div><div class="ledger-row"><dt>Importado</dt><dd class="mono">' + (c.importedAt ? dateBR(c.importedAt) : '—') + '</dd></div><div class="ledger-row"><dt>Lançamentos</dt><dd class="mono">' + (c.cnisCount || c.contribs || 0) + '</dd></div><div class="ledger-row"><dt>Ajustes</dt><dd class="mono">' + (c.adjustments || 0) + '</dd></div></dl><p class="form-note">Cada linha do relatório carrega a página de origem.</p></div></section>';
}

function viewCase() {
  const c = caseById(state.caseId);
  const cl = clientById(c.client);
  const head = '<div class="case-title"><div><h1 tabindex="-1">' + esc(c.name) + '</h1><div class="case-title__meta"><span>Cliente <strong>' + esc(cl.name) + '</strong></span><span>DIB <strong>' + dateBR(c.dib) + '</strong></span><span>Atualizado <strong>' + dateBR(c.updated) + '</strong></span></div></div><div class="page-head__actions"><button type="button" class="btn btn--quiet" data-nav="reports">Relatórios</button><button type="button" class="btn btn--primary" data-toast="Resultado recalculado na demonstração. Nenhum motor real foi executado.">' + ico('check', 15) + 'Recalcular</button></div></div>';
  const pass = '<div class="pass"><div class="pass__cell"><div class="pass__label">Cliente</div><div class="pass__value">' + esc(cl.name) + '</div></div><div class="pass__cell"><div class="pass__label">Módulo</div><div class="pass__value">' + esc(c.kind) + '</div></div><div class="pass__cell"><div class="pass__label">DIB</div><div class="pass__value pass__value--mono">' + dateBR(c.dib) + '</div></div><div class="pass__cell"><div class="pass__label">Regra</div><div class="pass__value" title="' + esc(c.rule) + '">' + esc(c.rule.split(',')[0]) + '</div></div><div class="pass__cell"><div class="pass__label">Situação</div><div class="pass__value">' + stateTag(c.status, statusTone(c.status)) + '</div></div></div>';

  if (state.caseTab === 'overview') {
    const body = '<div class="stack"><section class="folio"><div class="folio__head"><div><div class="folio__title">Régua de tempo</div><div class="folio__sub">Foque um período para ler origem e conferência.</div></div><span class="stamp ' + (c.status === 'Bloqueado' ? 'stamp--bad' : c.status === 'Calculado' ? '' : 'stamp--warn') + '">' + esc(c.status) + '</span></div><div class="folio__body">' + timeline(c) + '</div></section>' +
      '<div class="case-grid case-grid--inner"><section class="folio"><div class="folio__head"><div class="folio__title">Quadro de conferência</div></div><div class="folio__body"><dl class="ledger-list" style="margin:0"><div class="ledger-row"><dt>Lançamentos no CNIS</dt><dd class="mono">' + (c.cnisCount || c.contribs || 0) + '</dd></div><div class="ledger-row"><dt>Ajustes manuais</dt><dd class="mono">' + (c.adjustments || 0) + '</dd></div><div class="ledger-row"><dt>Usados no cálculo</dt><dd class="mono">' + (c.used || c.contribs || 0) + '</dd></div><div class="ledger-row"><dt>Pendentes</dt><dd class="mono" style="color:var(--gold)">' + (c.pending || 0) + '</dd></div></dl><p class="form-note">O quadro preserva o que veio do CNIS, o que foi ajustado e o que o motor usou.</p></div></section>' +
      '<section class="folio"><div class="folio__head"><div class="folio__title">Premissas declaradas</div></div><div class="folio__body"><dl class="ledger-list" style="margin:0"><div class="ledger-row"><dt>Sexo cadastral</dt><dd>' + esc(cl.sex || '—') + '</dd></div><div class="ledger-row"><dt>Deficiência</dt><dd>' + esc(c.disability || 'Não declarada') + '</dd></div><div class="ledger-row"><dt>CTC</dt><dd>' + esc(c.ctc || 'Não se aplica') + '</dd></div><div class="ledger-row"><dt>DIB</dt><dd class="mono">' + dateBR(c.dib) + '</dd></div></dl><p class="form-note">Premissa alterada invalida o resultado anterior e a revisão registrada.</p></div></section></div>' +
      '<div class="notice">' + ico('alert', 16) + '<p><strong>Resultado experimental.</strong> A hipótese organiza o trabalho e mostra a origem de cada número; ela não certifica direito a benefício.</p></div></div>';
    return head + pass + caseTabs() + '<div class="case-grid"><div>' + body + '</div><aside class="side-stack">' + resultPanel(c) + '</aside></div>';
  }

  if (state.caseTab === 'cnis') {
    const rows = pixelsFor(c).map((p) => '<tr><td><strong>' + esc(p.employer) + '</strong><div class="muted small">' + esc(p.cnpj) + '</div></td><td>' + esc(p.kind) + '</td><td class="mono nowrap">' + p.start + ' a ' + p.end + '</td><td class="num">' + p.contribs + '</td><td class="num">' + months(periodMonths(p)) + '</td><td>' + stateTag(p.status, p.status === 'pendente' || p.status === 'ajustado' ? 'warn' : 'ok') + '</td><td class="muted small">' + esc(p.source) + '</td></tr>').join('');
    return head + pass + caseTabs() + '<div class="case-grid"><div class="stack"><section class="folio"><div class="folio__head"><div><div class="folio__title">Vínculos importados</div><div class="folio__sub">' + (c.cnisPages ? 'Fonte: CNIS de ' + c.cnisPages + ' páginas' : 'Fonte: CNIS importado') + (c.importedAt ? ', importado em ' + dateBR(c.importedAt) : '') + '.</div></div><span class="state state--ok">' + (c.cnisCount || c.contribs || 0) + ' lançamentos</span></div><div class="folio__body folio__body--flush"><div class="table-wrap"><table class="table"><thead><tr><th>Empregador</th><th>Tipo</th><th>Período</th><th class="num">Contribuições</th><th class="num">Tempo</th><th>Estado</th><th>Origem</th></tr></thead><tbody>' + rows + '</tbody></table></div></div></section><div class="notice">' + ico('alert', 16) + '<p><strong>A tabela que continua na página seguinte sem repetir o cabeçalho é lida com o contexto herdado.</strong> Nenhum vínculo é criado por nome de segurado.</p></div></div><aside class="side-stack">' + resultPanel(c) + '</aside></div>';
  }

  if (state.caseTab === 'contributions') {
    const list = contribsFor(c);
    const rows = list.map((r) => '<tr><td class="mono">' + r.comp + '</td><td>' + esc(r.link) + '</td><td class="num">' + esc(r.value) + '</td><td>' + esc(r.origin) + '</td><td>' + stateTag(r.status, r.status === 'pendente' ? 'warn' : r.status === 'excluido' ? 'bad' : r.status === 'ajustado' ? 'warn' : 'ok') + '</td></tr>').join('');
    const pending = list.filter((r) => r.status === 'pendente').length;
    const table = list.length
      ? '<div class="table-wrap"><table class="table"><thead><tr><th>Competência</th><th>Vínculo</th><th class="num">Valor</th><th>Origem</th><th>Estado</th></tr></thead><tbody>' + rows + '</tbody></table></div>'
      : '<div class="empty"><strong>Nenhuma contribuição importada</strong><span>O livro de contribuições deste caso ainda não foi carregado no workspace.</span></div>';
    const badge = pending ? '<span class="state state--warn">' + pending + ' pendência' + (pending === 1 ? '' : 's') + '</span>' : '<span class="state state--ok">Sem pendências</span>';
    return head + pass + caseTabs() + '<div class="case-grid"><div class="stack"><section class="folio"><div class="folio__head"><div><div class="folio__title">Livro de contribuições</div><div class="folio__sub">Competência, valor e origem; exclusão documentada sem apagar o CNIS.</div></div>' + badge + '</div><div class="folio__body folio__body--flush">' + table + '</div></section><div class="notice">' + ico('lock', 16) + '<p><strong>Alterar um dado invalida o resultado e a revisão anterior.</strong> A aplicação aos salários do cálculo é explícita, nunca automática.</p></div></div><aside class="side-stack">' + resultPanel(c) + '</aside></div>';
  }

  if (state.caseTab === 'premises') {
    const rows = [
      ['Sexo cadastral', cl.sex || '—', 'Cadastro do cliente'], ['Data de nascimento', dateBR(cl.birth), 'Cadastro do cliente'],
      ['Deficiência', c.disability || 'Não declarada', 'Entrevista'], ['CTC', c.ctc || 'Não se aplica', '—'],
      ['DIB', dateBR(c.dib), 'Requerimento'], ['Regra testada', c.rule, 'Hipótese'],
      ['Tempo especial', c.special || 'Não considerado', 'Sem PPP'], ['Atividade rural', c.rural || 'Não declarada', '—']
    ].map((r) => '<div class="ledger-row"><dt>' + r[0] + '</dt><dd style="text-align:right">' + r[1] + '<div class="muted small" style="font-family:var(--font-ui)">' + r[2] + '</div></dd></div>').join('');
    return head + pass + caseTabs() + '<div class="case-grid"><div class="stack"><section class="folio"><div class="folio__head"><div><div class="folio__title">Premissas e origem</div><div class="folio__sub">O motor não confirma o que o documento não informa.</div></div></div><div class="folio__body"><dl class="ledger-list" style="margin:0">' + rows + '</dl></div></section><div class="notice">' + ico('alert', 16) + '<p><strong>Premissa não é fato.</strong> Cada linha declara de onde veio; mudar uma premissa muda o resultado e fica registrado na memória.</p></div></div><aside class="side-stack">' + resultPanel(c) + '</aside></div>';
  }

  if (state.caseTab === 'memory') {
    const rows = [
      ['Tempo de contribuição', '431 competências', 'CNIS + ajustes'],
      ['Carência', '431 contribuições', 'Livro de contribuições'],
      ['Média das remunerações', 'R$ 4.906,12', 'Competências usadas'],
      ['Coeficiente aplicado', '98,08%', 'Regra de transição'],
      ['Teto', 'R$ 8.157,41', 'Tabela de tetos'],
      ['RMI', 'R$ 4.812,37', 'Média × coeficiente']
    ].map((r) => '<div class="ledger-row"><dt>' + r[0] + '</dt><dd style="text-align:right"><span class="mono">' + r[1] + '</span><div class="muted small" style="font-family:var(--font-ui)">' + r[2] + '</div></dd></div>').join('');
    return head + pass + caseTabs() + '<div class="case-grid"><div class="stack"><section class="folio"><div class="folio__head"><div><div class="folio__title">Memória de cálculo</div><div class="folio__sub">Cada linha carrega a origem e a consequência.</div></div><span class="stamp">Revisada</span></div><div class="folio__body"><dl class="ledger-list" style="margin:0">' + rows + '</dl><p class="form-note">Fórmulas completas não são recuperadas do binário de referência; esta memória é uma demonstração de interface.</p></div></section><div class="notice">' + ico('check', 16) + '<p><strong>O resultado acompanha a memória.</strong> Um número sem origem não entra no relatório.</p></div></div><aside class="side-stack">' + resultPanel(c) + '</aside></div>';
  }

  const reports = REPORT_TYPES.map((r) => '<label class="report-row"><input type="checkbox" checked><div><strong>' + esc(r.name) + '</strong><span>' + esc(r.desc) + '</span></div><span class="state state--quiet">PDF</span></label>').join('');
  return head + pass + caseTabs() + '<div class="case-grid"><div class="stack"><section class="folio"><div class="folio__head"><div><div class="folio__title">Relatórios deste cálculo</div><div class="folio__sub">Oito documentos; escolha o que entra no lote.</div></div><button type="button" class="btn btn--primary" data-toast="Lote simulado. Na versão desktop, o PDF é gerado pelo renderer Rust.">' + ico('download', 15) + 'Gerar lote</button></div><div class="folio__body">' + reports + '</div></section></div><aside class="side-stack">' + resultPanel(c) + '</aside></div>';
}
