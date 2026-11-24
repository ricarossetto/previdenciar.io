function folio(title, sub, body, tools) {
  const flush = body.indexOf('<table') === 0 || body.indexOf('<div class="list"') === 0;
  return '<section class="folio"><div class="folio__head"><div><div class="folio__title">' + title + '</div>' + (sub ? '<div class="folio__sub">' + sub + '</div>' : '') + '</div>' + (tools || '') + '</div><div class="folio__body' + (flush ? ' folio__body--flush' : '') + '">' + body + '</div></section>';
}

function viewHome() {
  const active = activeCases();
  const reviewing = active.filter((c) => c.status === 'Em revisão' || c.status === 'Pendente').length;
  const blocked = active.filter((c) => c.status === 'Bloqueado').length;
  const recent = active.slice().sort((a, b) => b.updated.localeCompare(a.updated)).slice(0, 4);
  const agenda = APPOINTMENTS.filter((a) => a.date === TODAY);
  const rows = recent.map((c) => '<button type="button" class="list__item" data-case="' + c.id + '"><strong>' + esc(c.name) + '</strong><span>' + esc(clientById(c.client).name) + ' — ' + esc(c.kind) + ' — DIB ' + dateBR(c.dib) + '</span></button>').join('');
  const pendCases = CASES.filter((c) => c.status === 'Bloqueado' || c.status === 'Pendente' || c.status === 'Em revisão').slice(0, 3);
  const pend = pendCases.length ? pendCases.map((c) => {
    const p = pendenciesFor(c)[0] || { tone: 'warn', t: c.status, d: 'Revisão pendente.' };
    return { t: p.t, c: clientById(c.client).name + ' — ' + c.name, d: p.d, tone: c.status === 'Bloqueado' ? 'bad' : 'warn' };
  }) : [{ t: 'Nenhuma pendência aberta', c: 'Carteira', d: 'Os casos do workspace estão conferidos.', tone: 'ok' }];
  const pendHtml = pend.map((p) => '<div class="pendency pendency--' + (p.tone === 'bad' ? 'bad' : p.tone === 'ok' ? 'ok' : '') + '"><span class="pendency__mark"></span><div class="pendency__copy"><strong>' + esc(p.t) + '</strong><span>' + esc(p.c) + '. ' + esc(p.d) + '</span></div></div>').join('');
  const agendaHtml = agenda.map((a) => '<div class="appt"><time>' + a.time + '</time><div><strong>' + esc(a.title) + '</strong><span>' + esc(a.who) + '</span></div></div>').join('');
  const reports = REPORT_TYPES.slice(0, 3).map((r) => '<div class="ledger-row"><dt>' + esc(r.name) + '</dt><dd class="mono">out/2026</dd></div>').join('');
  const quality = [
    { l: 'Clientes', v: String(CLIENTS.length) },
    { l: 'Casos', v: String(CASES.length) },
    { l: 'Competências', v: String(CASES.reduce((sum, c) => sum + (Number(c.contribs) || 0), 0)) },
    { l: 'Compromissos', v: String(APPOINTMENTS.length) }
  ].map((q) => '<div class="ledger-row"><dt>' + q.l + '</dt><dd class="mono">' + q.v + '</dd></div>').join('');

  return pageHead('Início', 'A carteira do escritório, o que está em revisão e o que vence hoje. Tudo com dados fictícios.', '<button type="button" class="btn btn--primary" data-new-case>' + ico('plus', 16) + 'Novo cálculo</button>') +
    '<div class="brief"><span><strong>' + active.length + '</strong> casos ativos</span><span><strong>' + reviewing + '</strong> em revisão</span><span><strong>' + blocked + '</strong> bloqueado</span><span><strong>8</strong> relatórios no mês</span></div>' +
    '<div class="home-grid"><div class="stack">' +
      '<section class="folio"><div class="folio__head"><div><div class="folio__title">Continue de onde parou</div><div class="folio__sub">Casos locais, origem registrada e revisão ainda em aberto.</div></div><button type="button" class="btn btn--ghost" data-nav="cases">Ver cálculos' + ico('arrow', 15) + '</button></div><div class="folio__body folio__body--flush"><div class="list">' + rows + '</div></div></section>' +
      '<section class="folio"><div class="folio__head"><div><div class="folio__title">Pendências que travam o cálculo</div><div class="folio__sub">Dado ausente não vira zero.</div></div></div><div class="folio__body">' + pendHtml + '</div></section>' +
      '<section class="folio"><div class="folio__head"><div><div class="folio__title">Relatórios disponíveis</div><div class="folio__sub">Modelos previstos para o escritório, sem envio a servidor.</div></div><button type="button" class="btn btn--ghost" data-nav="reports">Abrir central' + ico('arrow', 15) + '</button></div><div class="folio__body"><dl class="ledger-list" style="margin:0">' + reports + '</dl></div></section>' +
    '</div><aside class="side-stack">' +
      '<section class="folio"><div class="folio__head"><div class="folio__title">Hoje, 26 de outubro</div><span class="folio__sub">' + agenda.length + ' compromissos</span></div><div class="folio__body">' + agendaHtml + '<div class="form-actions" style="justify-content:flex-start"><button type="button" class="btn btn--quiet" data-nav="agenda">Abrir agenda</button></div></div></section>' +
      '<section class="folio"><div class="folio__head"><div class="folio__title">Carteira</div></div><div class="folio__body"><dl class="ledger-list" style="margin:0">' + quality + '</dl><p class="form-note">Contagens do workspace atual. A conferência real é feita documento a documento.</p></div></section>' +
      '<div class="notice">' + ico('lock', 16) + '<p><strong>Base local, demonstração.</strong> O motor local não envia dados; a Fábrica de Cálculos usa o proxy local quando aberta. As telas de exportação são simulações.</p></div>' +
    '</aside></div>';
}

function viewClients() {
  const rows = CLIENTS.map((c) => {
    const n = caseCount(c.id);
    return '<tr data-search="' + esc((c.name + ' ' + c.cpf + ' ' + c.email + ' ' + c.phone).toLowerCase()) + '">' +
      '<td><button type="button" class="rowbtn" data-client="' + c.id + '"><strong>' + esc(c.name) + '</strong></button><div class="muted small">Nascimento ' + dateBR(c.birth) + '</div></td>' +
      '<td class="mono">' + esc(c.cpf) + '</td>' +
      '<td>' + esc(c.email) + '<div class="muted small">' + esc(c.phone) + '</div></td>' +
      '<td class="num">' + n + '</td>' +
      '<td>' + stateTag('Ativo', 'ok') + '</td>' +
      '<td><div class="inline-actions"><button type="button" class="btn btn--ghost" data-client="' + c.id + '">Abrir</button><button type="button" class="btn btn--ghost" data-new-case="' + c.id + '">Novo cálculo</button></div></td>' +
      '</tr>';
  }).join('');

  return pageHead('Clientes', 'Cadastro local da carteira, com origem declarada.', '<button type="button" class="btn btn--primary" data-new-case>' + ico('plus', 16) + 'Novo cliente</button>') +
    '<div class="toolbar"><div class="searchbox">' + ico('search', 16) + '<input id="filterClients" type="search" placeholder="Pesquisar nome, CPF, e-mail ou telefone" aria-label="Pesquisar clientes"></div><span class="count" id="clientsCount">' + CLIENTS.length + ' clientes</span></div>' +
    '<section class="folio"><div class="folio__body folio__body--flush"><div class="table-wrap"><table class="table" id="clientsTable"><thead><tr><th>Cliente</th><th>CPF / CNPJ</th><th>Contato</th><th class="num">Cálculos</th><th>Situação</th><th>Ações</th></tr></thead><tbody>' + rows + '</tbody></table></div></div></section>';
}

function caseRows(list) {
  return list.map((c) => '<tr data-search="' + esc((c.name + ' ' + clientById(c.client).name + ' ' + c.kind + ' ' + c.status).toLowerCase()) + '" data-kind="' + esc(c.kind) + '" data-status="' + esc(c.status) + '">' +
    '<td><button type="button" class="rowbtn" data-case="' + c.id + '"><strong>' + esc(c.name) + '</strong></button><div class="muted small">' + esc(clientById(c.client).name) + '</div></td>' +
    '<td>' + esc(c.kind) + '</td><td class="mono nowrap">' + dateBR(c.dib) + '</td>' +
    '<td>' + stateTag(c.status, statusTone(c.status)) + '</td>' +
    '<td class="num">' + (c.rmi == null ? '—' : brl(c.rmi)) + '</td>' +
    '<td><button type="button" class="btn btn--ghost" data-case="' + c.id + '">Abrir</button></td></tr>').join('');
}

function viewCases() {
  return pageHead('Cálculos', 'Hipóteses separadas por cliente e módulo. O resultado nunca apaga a origem nem a revisão anterior.', '<button type="button" class="btn btn--primary" data-new-case>' + ico('plus', 16) + 'Novo cálculo</button>') +
    '<div class="toolbar"><div class="searchbox">' + ico('search', 16) + '<input id="filterCases" type="search" placeholder="Pesquisar nome, cliente ou situação" aria-label="Pesquisar cálculos"></div>' +
    '<select class="select" id="filterKind" aria-label="Filtrar módulo"><option>Todos os módulos</option><option>RGPS</option><option>PcD</option></select>' +
    '<select class="select" id="filterStatus" aria-label="Filtrar situação"><option>Todas as situações</option><option>Calculado</option><option>Em revisão</option><option>Pendente</option><option>Bloqueado</option><option>Rascunho</option></select>' +
    '<span class="count" id="casesCount">' + activeCases().length + ' cálculos</span></div>' +
    '<section class="folio"><div class="folio__body folio__body--flush"><div class="table-wrap"><table class="table" id="casesTable"><thead><tr><th>Cálculo</th><th>Módulo</th><th>DIB</th><th>Situação</th><th class="num">RMI</th><th>Ações</th></tr></thead><tbody>' + caseRows(activeCases()) + '</tbody></table></div></div></section>' +
    '<div class="notice" style="margin-top:14px">' + ico('alert', 16) + '<p><strong>Hipótese bloqueada continua bloqueada.</strong> Nenhum caso desta demonstração certifica direito; os valores são fictícios e servem para mostrar o fluxo.</p></div>';
}

function viewRGPS() {
  const c = CASES[0];
  const cl = clientById(c.client);
  const age = ageAt(cl.birth, TODAY);
  const years = Math.floor((Number(c.months) || 0) / 12);
  const points = Number.isFinite(age) ? age + years : null;
  const rows = RGPS_SCENARIOS.map((s) => '<tr><td><button type="button" class="rowbtn" data-rgps="' + s.id + '"><strong>' + esc(s.name) + '</strong></button></td><td>' + esc(s.req) + '</td><td>' + esc(s.pos) + '</td><td>' + stateTag(s.status, s.tone) + '</td><td class="num">' + esc(s.rmi) + '</td><td><button type="button" class="btn btn--ghost" data-rgps="' + s.id + '">Detalhar</button></td></tr>').join('');
  const sel = RGPS_SCENARIOS.find((s) => s.id === state.rgpsScenario) || RGPS_SCENARIOS[1];
  const detail = '<section class="folio"><div class="folio__head"><div><div class="folio__title">' + esc(sel.name) + '</div><div class="folio__sub">Cenário selecionado para ' + esc(cl.name) + '</div></div>' + stateTag(sel.status, sel.tone) + '</div><div class="folio__body"><dl class="ledger-list" style="margin:0"><div class="ledger-row"><dt>Requisito</dt><dd style="text-align:right">' + esc(sel.req) + '</dd></div><div class="ledger-row"><dt>Posição da cliente</dt><dd style="text-align:right">' + esc(sel.pos) + '</dd></div><div class="ledger-row"><dt>RMI estimada</dt><dd class="gold">' + esc(sel.rmi) + '</dd></div></dl><div class="form-actions"><button type="button" class="btn btn--quiet" data-toast="Comparação salva como cenário no caso fictício.">Salvar cenário</button><button type="button" class="btn btn--primary" data-case="1">Abrir o caso</button></div></div></section>';
  return pageHead('Cálculos do RGPS', 'Compare regras de transição com a posição do segurado. A escolha é profissional; o assistente não decide por você.') +
    '<div class="brief"><span>Cliente <strong>' + esc(cl.name) + '</strong></span><span>Idade <strong>' + (Number.isFinite(age) ? age + ' anos' : '—') + '</strong></span><span>Tempo <strong>' + months(c.months) + '</strong></span><span>Pontos <strong>' + (points == null ? '—' : points) + '</strong></span></div>' +
    '<section class="folio"><div class="folio__head"><div><div class="folio__title">Cenários de transição</div><div class="folio__sub">Regras ilustrativas para 2026; não certificam direito.</div></div></div><div class="folio__body folio__body--flush"><div class="table-wrap"><table class="table"><thead><tr><th>Cenário</th><th>Requisito</th><th>Posição</th><th>Situação</th><th class="num">RMI</th><th>Ação</th></tr></thead><tbody>' + rows + '</tbody></table></div></div></section>' +
    '<div class="case-grid" style="margin-top:14px"><div>' + detail + '</div><div class="notice">' + ico('alert', 16) + '<p><strong>Cenário ilustrativo.</strong> A elegibilidade real depende de documentos, CNIS conferido e revisão profissional. O mock não executa o motor.</p></div></div>';
}
