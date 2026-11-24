function renderNav() {
  let html = '';
  let group = null;
  NAV.forEach((n) => {
    if (n.group && n.group !== group) {
      group = n.group;
      html += '<div class="nav__group">' + esc(group) + '</div>';
    }
    const active = state.view === n.id || (state.view === 'case' && n.id === 'cases');
    html += '<button type="button" class="nav__link" data-nav="' + n.id + '"' + (active ? ' aria-current="page"' : '') + '>' + ico(n.icon, 16) + '<span>' + esc(n.label) + '</span>' + navCount(n.id) + '</button>';
  });
  $('#nav').innerHTML = html;
  $('#railToggle').setAttribute('aria-expanded', String(state.railOpen));
  $('#rail').setAttribute('data-open', String(state.railOpen));
  $('#scrim').hidden = !state.railOpen;
  $$('.dock button[data-nav]').forEach((b) => {
    const on = state.view === b.dataset.nav || (state.view === 'case' && b.dataset.nav === 'cases');
    if (on) b.setAttribute('aria-current', 'page'); else b.removeAttribute('aria-current');
  });
}

function crumb() {
  if (state.view === 'case') return 'Cálculo: ' + caseById(state.caseId).name;
  const n = NAV.find((x) => x.id === state.view);
  return n ? n.label : 'Início';
}

function render() {
  document.documentElement.setAttribute('data-theme', state.theme);
  document.documentElement.setAttribute('data-motion', state.motion);
  if (!CASES.some((c) => c.id === state.caseId)) state.caseId = CASES[0].id;
  if (!CASES.some((c) => c.id === state.reportCase)) state.reportCase = CASES[0].id;
  const imported = state.workspaceSource === 'imported';
  $('#srcBadge').textContent = imported ? 'Dados locais' : 'Demonstração';
  $('#srcBadge').dataset.source = imported ? 'imported' : 'demo';
  $('#railNote').innerHTML = imported ? 'Base local · dados importados<br>Motor 0.10.5 · sem execução' : 'Base local · demonstração<br>Motor 0.10.5 · dados fictícios';
  $('#footNote').textContent = imported ? 'Dados locais importados. O motor local não envia dados; a Fábrica usa o proxy local quando aberta.' : 'Dados fictícios. O motor local não envia dados; a Fábrica usa o proxy local quando aberta.';
  $('#crumb').textContent = crumb();
  renderNav();
  const views = {
    home: viewHome, clients: viewClients, cases: viewCases, rgps: viewRGPS, engine: viewEngine, case: viewCase,
    fabrica: viewFabrica,
    assistant: viewAssistant, editor: viewEditor, tables: viewTables, modules: viewModules,
    documents: viewDocuments, reports: viewReports, agenda: viewAgenda, settings: viewSettings, coverage: viewCoverage
  };
  $('#view').innerHTML = (views[state.view] || viewHome)();
  if (state.view === 'engine') initEnginePage();
  if (state.view === 'fabrica') initFabricaPage();
  document.title = crumb() + ' · previdenciar.io';
  $('#themeBtn').setAttribute('aria-pressed', String(state.theme === 'dark'));
  $('#motionBtn').setAttribute('aria-pressed', String(state.motion === 'reduced'));
}

function go(view, opts) {
  opts = opts || {};
  if (view === 'case') {
    state.caseId = opts.caseId || state.caseId;
    state.caseTab = opts.tab || 'overview';
  }
  state.view = view;
  state.railOpen = false;
  render();
  const hash = view === 'case' ? 'case/' + state.caseId + '/' + state.caseTab : view;
  try { history.pushState(null, '', '#' + hash); } catch (e) {}
  const h = $('#view h1');
  if (h) h.focus({ preventScroll: true });
  window.scrollTo({ top: 0, behavior: state.motion === 'reduced' ? 'auto' : 'smooth' });
}

function parseHash() {
  const h = location.hash.replace(/^#/, '');
  if (!h) return;
  if (h.indexOf('case/') === 0) {
    const p = h.split('/');
    state.caseId = Number(p[1]) || 1;
    state.caseTab = p[2] || 'overview';
    state.view = 'case';
    return;
  }
  if (NAV.some((n) => n.id === h)) state.view = h;
}

function toast(message, kind) {
  const t = $('#toast');
  t.innerHTML = ico(kind === 'error' ? 'alert' : 'check', 16) + '<span>' + esc(message) + '</span>';
  t.dataset.kind = kind || 'ok';
  t.setAttribute('role', kind === 'error' ? 'alert' : 'status');
  t.setAttribute('aria-live', kind === 'error' ? 'assertive' : 'polite');
  t.hidden = false;
  clearTimeout(toast._timer);
  toast._timer = setTimeout(() => { t.hidden = true; }, 3600);
}

function openModal(html) {
  $('#modalRoot').innerHTML = '<div class="overlay" id="modalOverlay"><div class="modal" role="dialog" aria-modal="true">' + html + '</div></div>';
  document.body.style.overflow = 'hidden';
  const focusable = $('#modalRoot button, #modalRoot input, #modalRoot select, #modalRoot textarea');
  if (focusable) focusable.focus();
}

function closeModal() {
  $('#modalRoot').innerHTML = '';
  document.body.style.overflow = '';
}

function openNewCase(clientId) {
  const c = clientId ? clientById(Number(clientId)) : null;
  openModal('<div class="modal__head"><div><h2>Novo cálculo</h2><p>' + (c ? 'Para ' + esc(c.name) + '. ' : '') + 'Cada cálculo é separado e preserva a origem.</p></div><button type="button" class="iconbtn" data-close-modal aria-label="Fechar">' + ico('close', 16) + '</button></div><div class="modal__body"><div class="choice-cards"><button type="button" class="choice-card" data-start="assistant"><em>Recomendado</em><strong>Preenchimento guiado</strong><span>Entrevista em sete etapas, importação do CNIS e conferência antes do cálculo.</span><ul class="choice-card__list"><li>O assistente pergunta e registra cada resposta.</li><li>Nenhum dado ausente vira zero.</li></ul></button><button type="button" class="choice-card" data-start="editor"><em>Editor direto</em><strong>Preenchimento manual</strong><span>Dados nos painéis do editor, com origem e limites preservados.</span><ul class="choice-card__list"><li>Para quem já conhece o caso.</li><li>Alterar um dado invalida o resultado anterior.</li></ul></button></div><p class="form-note">Demonstração: nenhum cálculo real é executado e nenhum arquivo é gravado.</p></div>');
}

function openClient(id) {
  const c = clientById(Number(id));
  const cases = CASES.filter((x) => x.client === c.id).map((x) => '<div class="ledger-row"><dt>' + esc(x.name) + '</dt><dd><button type="button" class="linkbtn" data-case="' + x.id + '">Abrir</button></dd></div>').join('');
  openModal('<div class="modal__head"><div><h2>' + esc(c.name) + '</h2><p>Cadastro local · ' + esc(c.city) + '</p></div><button type="button" class="iconbtn" data-close-modal aria-label="Fechar">' + ico('close', 16) + '</button></div><div class="modal__body"><div class="kv-grid"><div class="kv"><dt>CPF / CNPJ</dt><dd class="mono">' + esc(c.cpf) + '</dd></div><div class="kv"><dt>Nascimento</dt><dd>' + dateBR(c.birth) + '</dd></div><div class="kv"><dt>E-mail</dt><dd>' + esc(c.email) + '</dd></div><div class="kv"><dt>Telefone</dt><dd>' + esc(c.phone) + '</dd></div></div><p class="form-note">' + esc(c.notes) + '</p><div class="hr"></div><div class="folio__title" style="margin-bottom:6px">Cálculos</div>' + (cases || '<p class="muted">Nenhum cálculo ainda.</p>') + '</div><div class="modal__actions"><button type="button" class="btn btn--quiet" data-close-modal>Fechar</button><button type="button" class="btn btn--primary" data-new-case="' + c.id + '">Novo cálculo</button></div>');
}

function openAllPages() {
  const list = NAV.map((n) => '<button type="button" class="palette__item" data-nav="' + n.id + '">' + ico(n.icon, 17) + '<span>' + esc(n.label) + '</span><em>área</em></button>').join('');
  openModal('<div class="modal__head"><div><h2>Todas as páginas</h2><p>Todas as áreas do previdenciar.io, também no celular.</p></div><button type="button" class="iconbtn" data-close-modal aria-label="Fechar">' + ico('close', 16) + '</button></div><div class="modal__body"><div class="palette__list" style="max-height:60vh">' + list + '</div></div>');
}

function paletteHtml() {
  const navItems = NAV.map((n) => '<button type="button" class="palette__item" data-palette="nav:' + n.id + '" data-search="' + esc(n.label.toLowerCase()) + '">' + ico(n.icon, 17) + '<span>' + esc(n.label) + '</span><em>área</em></button>').join('');
  const clientItems = CLIENTS.map((c) => '<button type="button" class="palette__item" data-palette="client:' + c.id + '" data-search="' + esc((c.name + ' ' + c.email).toLowerCase()) + '">' + ico('users', 17) + '<span>' + esc(c.name) + '</span><em>cliente</em></button>').join('');
  const caseItems = CASES.map((c) => '<button type="button" class="palette__item" data-palette="case:' + c.id + '" data-search="' + esc((c.name + ' ' + clientById(c.client).name).toLowerCase()) + '">' + ico('calc', 17) + '<span>' + esc(c.name) + ' — ' + esc(clientById(c.client).name) + '</span><em>cálculo</em></button>').join('');
  return '<div class="overlay" id="paletteOverlay"><div class="palette" role="dialog" aria-modal="true" aria-label="Buscar no escritório"><div class="palette__input">' + ico('search', 18) + '<input id="paletteInput" type="search" placeholder="Cliente, cálculo ou área" autocomplete="off" aria-label="Buscar"></div><div class="palette__list" id="paletteList"><div class="palette__group">Áreas</div>' + navItems + '<div class="palette__group">Clientes</div>' + clientItems + '<div class="palette__group">Cálculos</div>' + caseItems + '<div class="empty" id="paletteEmpty" hidden><strong>Nada encontrado</strong><span>Tente outro nome ou abra uma área.</span></div></div><div class="palette__foot">Enter abre o primeiro resultado · Esc fecha · Ctrl K abre de novo</div></div></div>';
}

function openPalette() {
  if ($('#paletteOverlay')) return;
  document.body.insertAdjacentHTML('beforeend', paletteHtml());
  document.body.style.overflow = 'hidden';
  const input = $('#paletteInput');
  input.value = '';
  input.focus();
  filterPalette('');
}

function closePalette() {
  const el = $('#paletteOverlay');
  if (el) el.remove();
  document.body.style.overflow = '';
}

function filterPalette(q) {
  const term = (q || '').trim().toLowerCase();
  const items = $$('#paletteList .palette__item');
  let visible = 0;
  items.forEach((it) => {
    const ok = !term || (it.dataset.search || '').indexOf(term) >= 0;
    it.hidden = !ok;
    it.removeAttribute('data-active');
    if (ok) visible++;
  });
  $$('#paletteList .palette__group').forEach((g) => { g.hidden = !!term; });
  const first = items.find((it) => !it.hidden);
  if (first) first.setAttribute('data-active', 'true');
  const empty = $('#paletteEmpty');
  if (empty) empty.hidden = visible > 0;
  const active = $('#paletteList .palette__item[data-active="true"]');
  if (active) active.scrollIntoView({ block: 'nearest' });
}

function movePalette(delta) {
  const items = $$('#paletteList .palette__item').filter((it) => !it.hidden);
  if (!items.length) return;
  let i = items.findIndex((it) => it.dataset.active === 'true');
  i = (i + delta + items.length) % items.length;
  items.forEach((it) => it.removeAttribute('data-active'));
  items[i].setAttribute('data-active', 'true');
  items[i].scrollIntoView({ block: 'nearest' });
}

function runPalette(value) {
  if (!value) return;
  const parts = value.split(':');
  const kind = parts[0];
  const id = parts[1];
  closePalette();
  if (kind === 'nav') go(id);
  else if (kind === 'client') openClient(Number(id));
  else if (kind === 'case') go('case', { caseId: Number(id) });
}

function periodReadoutHtml(p) {
  return '<strong>' + esc(p.employer) + '</strong> — ' + esc(p.kind) + ' — ' + p.start + ' a ' + p.end + ' — ' + months(periodMonths(p)) + ' — ' + p.contribs + ' contribuições — ' + esc(p.status) + ' — ' + esc(p.source);
}

function filterClientRows() {
  const q = ($('#filterClients') ? $('#filterClients').value : '').toLowerCase();
  const rows = $$('#clientsTable tbody tr');
  let n = 0;
  rows.forEach((r) => {
    const ok = !q || (r.dataset.search || '').indexOf(q) >= 0;
    r.hidden = !ok;
    if (ok) n++;
  });
  const c = $('#clientsCount');
  if (c) c.textContent = n + ' clientes';
}

function filterCaseRows() {
  const q = ($('#filterCases') ? $('#filterCases').value : '').toLowerCase();
  const kind = $('#filterKind') ? $('#filterKind').value : 'Todos os módulos';
  const status = $('#filterStatus') ? $('#filterStatus').value : 'Todas as situações';
  const rows = $$('#casesTable tbody tr');
  let n = 0;
  rows.forEach((r) => {
    let ok = !q || (r.dataset.search || '').indexOf(q) >= 0;
    if (ok && kind !== 'Todos os módulos') ok = r.dataset.kind === kind;
    if (ok && status !== 'Todas as situações') ok = r.dataset.status === status;
    r.hidden = !ok;
    if (ok) n++;
  });
  const c = $('#casesCount');
  if (c) c.textContent = n + ' cálculos';
}

document.addEventListener('click', (e) => {
  const nav = e.target.closest('[data-nav]');
  if (nav) { e.preventDefault(); closeModal(); go(nav.dataset.nav); return; }
  const start = e.target.closest('[data-start]');
  if (start) { const to = start.dataset.start; closeModal(); go(to); return; }
  const c = e.target.closest('[data-case]');
  if (c) { e.preventDefault(); closeModal(); go('case', { caseId: Number(c.dataset.case) }); return; }
  const tab = e.target.closest('[data-tab]');
  if (tab) {
    state.caseTab = tab.dataset.tab;
    render();
    const t = $('[data-tab="' + state.caseTab + '"]');
    if (t) t.focus();
    try { history.replaceState(null, '', '#case/' + state.caseId + '/' + state.caseTab); } catch (err) {}
    return;
  }
  const client = e.target.closest('[data-client]');
  if (client) { e.preventDefault(); openClient(Number(client.dataset.client)); return; }
  const newCase = e.target.closest('[data-new-case]');
  if (newCase) { e.preventDefault(); openNewCase(newCase.dataset.newCase); return; }
  const t = e.target.closest('[data-toast]');
  if (t) { e.preventDefault(); toast(t.dataset.toast); return; }
  const rgps = e.target.closest('[data-rgps]');
  if (rgps) { state.rgpsScenario = Number(rgps.dataset.rgps); render(); return; }
  const choice = e.target.closest('[data-choice]');
  if (choice) {
    state.assistantChoice = Number(choice.dataset.choice);
    $$('.choice').forEach((b) => {
      const on = Number(b.dataset.choice) === state.assistantChoice;
      b.setAttribute('aria-pressed', String(on));
      const mark = b.querySelector('.choice__mark');
      if (mark) mark.textContent = on ? '✓' : '';
    });
    return;
  }
  if (e.target.closest('[data-assistant-next]')) {
    if (state.assistantStep < 6) { state.assistantStep++; state.assistantChoice = null; render(); const h = $('#view h1'); if (h) h.focus(); toast('Etapa registrada na demonstração.'); }
    else toast('Demonstração: conferência concluída. O resultado segue experimental.');
    return;
  }
  if (e.target.closest('[data-assistant-back]')) {
    if (state.assistantStep > 0) { state.assistantStep--; state.assistantChoice = null; render(); const h = $('#view h1'); if (h) h.focus(); }
    return;
  }
  const series = e.target.closest('[data-series]');
  if (series) { state.seriesId = series.dataset.series; render(); return; }
  const doc = e.target.closest('[data-doc]');
  if (doc) { state.docId = Number(doc.dataset.doc); render(); return; }
  if (e.target.closest('[data-toggle-theme]')) { state.theme = state.theme === 'dark' ? 'light' : 'dark'; persist(); render(); return; }
  if (e.target.closest('[data-toggle-motion]')) { state.motion = state.motion === 'reduced' ? 'full' : 'reduced'; persist(); render(); return; }
  if (e.target.closest('[data-report-batch]')) {
    const n = $$('#view .report-row input[type="checkbox"]:checked').length;
    toast(n + ' relatório' + (n === 1 ? '' : 's') + ' no lote simulado. Nenhum PDF foi gerado.');
    return;
  }
  if (e.target.closest('[data-engine-example]')) { runEngine('example'); return; }
  if (e.target.closest('[data-engine-case]')) { runEngine('case'); return; }
  if (e.target.closest('[data-export]')) { downloadWorkspace(); return; }
  if (e.target.closest('[data-import-trigger]')) { const f = $('#importFile'); if (f) f.click(); return; }
  if (e.target.closest('[data-restore-demo]')) { restoreDemo(); return; }
  if (e.target.closest('[data-print]')) { window.print(); return; }
  if (e.target.closest('#searchBtn')) { openPalette(); return; }
  if (e.target.closest('#themeBtn')) { state.theme = state.theme === 'dark' ? 'light' : 'dark'; persist(); render(); return; }
  if (e.target.closest('#motionBtn')) { state.motion = state.motion === 'reduced' ? 'full' : 'reduced'; persist(); render(); return; }
  if (e.target.closest('#railToggle')) { state.railOpen = !state.railOpen; renderNav(); return; }
  if (e.target.closest('#scrim')) { state.railOpen = false; renderNav(); return; }
  if (e.target.closest('#dockAll')) { openAllPages(); return; }
  if (e.target.closest('[data-close-modal]')) { closeModal(); return; }
  if (e.target.id === 'modalOverlay') { closeModal(); return; }
  if (e.target.id === 'paletteOverlay') { closePalette(); return; }
  const p = e.target.closest('[data-palette]');
  if (p) { runPalette(p.dataset.palette); return; }
});

document.addEventListener('input', (e) => {
  if (e.target.id === 'filterClients') filterClientRows();
  if (e.target.id === 'filterCases') filterCaseRows();
  if (e.target.id === 'paletteInput') filterPalette(e.target.value);
});

document.addEventListener('change', (e) => {
  if (e.target.id === 'filterKind' || e.target.id === 'filterStatus') filterCaseRows();
  if (e.target.id === 'reportCase') {
    state.reportCase = Number(e.target.value) || CASES[0].id;
    render();
    const s = $('#reportCase');
    if (s) s.focus();
  }
  if (e.target.id === 'importFile') {
    const file = e.target.files && e.target.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      try { importWorkspaceText(String(reader.result || '')); }
      catch (err) { toast('Não foi possível importar: ' + err.message, 'error'); }
    };
    reader.onerror = () => toast('Não foi possível ler o arquivo.', 'error');
    reader.readAsText(file, 'utf-8');
    e.target.value = '';
  }
});

document.addEventListener('pointerover', (e) => {
  const p = e.target.closest('[data-period]');
  if (!p) return;
  const c = caseById(state.caseId);
  const list = pixelsFor(c);
  const out = $('#periodReadout');
  if (out && list[Number(p.dataset.period)]) out.innerHTML = periodReadoutHtml(list[Number(p.dataset.period)]);
});
document.addEventListener('focusin', (e) => {
  const p = e.target.closest('[data-period]');
  if (!p) return;
  const c = caseById(state.caseId);
  const list = pixelsFor(c);
  const out = $('#periodReadout');
  if (out && list[Number(p.dataset.period)]) out.innerHTML = periodReadoutHtml(list[Number(p.dataset.period)]);
});
document.addEventListener('pointerout', (e) => {
  const tl = e.target.closest('.tl');
  if (!tl) return;
  if (e.relatedTarget && tl.contains(e.relatedTarget)) return;
  const out = $('#periodReadout');
  if (out) out.textContent = 'Foque um período para ver origem, tempo e contribuições.';
});

document.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); openPalette(); return; }
  if (e.key === 'Escape') {
    if ($('#paletteOverlay')) { closePalette(); return; }
    if ($('#modalRoot').children.length) { closeModal(); return; }
    if (state.railOpen) { state.railOpen = false; renderNav(); }
    return;
  }
  if ($('#paletteOverlay')) {
    if (e.key === 'ArrowDown') { e.preventDefault(); movePalette(1); }
    if (e.key === 'ArrowUp') { e.preventDefault(); movePalette(-1); }
    if (e.key === 'Enter') {
      const active = $('#paletteList .palette__item[data-active="true"]');
      if (active) { e.preventDefault(); runPalette(active.dataset.palette); }
    }
  }
});

function persist() {
  try {
    localStorage.setItem('pio-theme', state.theme);
    localStorage.setItem('pio-motion', state.motion);
  } catch (e) {}
}

function init() {
  try {
    const t = localStorage.getItem('pio-theme');
    if (t === 'dark' || t === 'light') state.theme = t;
    const m = localStorage.getItem('pio-motion');
    if (m === 'reduced' || m === 'full') state.motion = m;
  } catch (e) {}
  loadSavedWorkspace();
  parseHash();
  render();
  window.addEventListener('popstate', () => { parseHash(); render(); });
}

init();
