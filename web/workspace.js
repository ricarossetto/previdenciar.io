const WORKSPACE_KEY = 'pio-workspace';
const DEMO = JSON.parse(JSON.stringify({
  version: 1,
  clients: CLIENTS,
  cases: CASES,
  periods: PERIODS,
  contributions: CONTRIBUTIONS,
  appointments: APPOINTMENTS,
  documents: DOCUMENTS
}));

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

function contribsFor(c) {
  if (!c) return [];
  return CONTRIBUTIONS[String(c.id)] || [];
}

function fail(message) {
  throw new Error(message);
}

function month(value, label) {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}$/.test(value)) fail(label + ' deve estar no formato AAAA-MM.');
  return value;
}

function day(value, label) {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(value)) fail(label + ' deve estar no formato AAAA-MM-DD.');
  return value;
}

function normalizeWorkspace(raw) {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) fail('O arquivo precisa ser um objeto JSON.');
  if (Number(raw.version) !== 1) fail('Versão de workspace não suportada. Use a versão 1.');
  if (!Array.isArray(raw.clients) || !raw.clients.length) fail('O workspace precisa de pelo menos um cliente.');
  if (!Array.isArray(raw.cases) || !raw.cases.length) fail('O workspace precisa de pelo menos um cálculo.');

  const clients = raw.clients.map((c, i) => {
    if (!c || typeof c.name !== 'string' || !c.name.trim()) fail('Cliente ' + (i + 1) + ' sem nome.');
    return Object.assign({}, c, { id: Number(c.id == null ? i + 1 : c.id) });
  });
  const clientIds = new Set(clients.map((c) => c.id));
  if (clientIds.size !== clients.length) fail('Há ids de cliente repetidos.');

  const cases = raw.cases.map((c, i) => {
    if (!c || typeof c.name !== 'string' || !c.name.trim()) fail('Cálculo ' + (i + 1) + ' sem nome.');
    const client = Number(c.client);
    if (!clientIds.has(client)) fail('O cálculo "' + c.name + '" aponta para um cliente inexistente.');
    return Object.assign({}, c, {
      id: Number(c.id == null ? i + 1 : c.id),
      client: client,
      kind: c.kind || 'RGPS',
      rule: c.rule || 'Hipótese',
      status: c.status || 'Rascunho',
      rmi: Number.isFinite(Number(c.rmi)) ? Number(c.rmi) : null,
      months: Number(c.months) || 0,
      contribs: Number(c.contribs) || 0,
      start: month(c.start, 'Início do cálculo "' + c.name + '"'),
      end: month(c.end, 'Fim do cálculo "' + c.name + '"'),
      dib: day(c.dib, 'DIB do cálculo "' + c.name + '"'),
      updated: c.updated || new Date().toISOString().slice(0, 10)
    });
  });
  const caseIds = new Set(cases.map((c) => c.id));
  if (caseIds.size !== cases.length) fail('Há ids de cálculo repetidos.');

  const periods = {};
  if (raw.periods && typeof raw.periods === 'object' && !Array.isArray(raw.periods)) {
    Object.keys(raw.periods).forEach((key) => {
      const id = Number(key);
      if (!caseIds.has(id)) return;
      const list = raw.periods[key];
      if (!Array.isArray(list)) fail('Os períodos do cálculo ' + key + ' precisam ser uma lista.');
      periods[String(id)] = list.map((p, i) => {
        if (!p || typeof p.employer !== 'string' || !p.employer.trim()) fail('Período ' + (i + 1) + ' do cálculo ' + key + ' sem empregador.');
        return Object.assign({}, p, {
          kind: p.kind || 'Empregado',
          status: p.status || 'conferido',
          source: p.source || 'Importado',
          cnpj: p.cnpj || '—',
          contribs: Number(p.contribs) || 0,
          start: month(p.start, 'Início de um período'),
          end: month(p.end, 'Fim de um período')
        });
      });
    });
  }

  const contributions = {};
  if (raw.contributions && typeof raw.contributions === 'object' && !Array.isArray(raw.contributions)) {
    Object.keys(raw.contributions).forEach((key) => {
      const id = Number(key);
      if (!caseIds.has(id)) return;
      const list = raw.contributions[key];
      if (!Array.isArray(list)) fail('As contribuições do cálculo ' + key + ' precisam ser uma lista.');
      contributions[String(id)] = list.map((r, i) => {
        if (!r || typeof r.comp !== 'string') fail('Contribuição ' + (i + 1) + ' do cálculo ' + key + ' sem competência.');
        return Object.assign({}, r, {
          link: r.link || '—',
          value: r.value == null ? '—' : String(r.value),
          origin: r.origin || 'Importado',
          status: r.status || 'conferido'
        });
      });
    });
  }

  const appointments = Array.isArray(raw.appointments) ? raw.appointments.map((a, i) => {
    if (!a || typeof a.title !== 'string') fail('Compromisso ' + (i + 1) + ' sem título.');
    return Object.assign({}, a, { date: a.date || new Date().toISOString().slice(0, 10), time: a.time || '09:00', who: a.who || '—' });
  }) : [];

  const documents = Array.isArray(raw.documents) ? raw.documents.map((d, i) => {
    if (!d || typeof d.name !== 'string') fail('Documento ' + (i + 1) + ' sem nome.');
    return Object.assign({}, d, { id: Number(d.id == null ? i + 1 : d.id), category: d.category || 'Geral', updated: d.updated || new Date().toISOString().slice(0, 10), body: d.body || '' });
  }) : [];

  return { version: 1, exportedAt: raw.exportedAt || new Date().toISOString(), clients: clients, cases: cases, periods: periods, contributions: contributions, appointments: appointments, documents: documents };
}

function workspacePayload() {
  return {
    version: 1,
    exportedAt: new Date().toISOString(),
    app: 'previdenciar.io',
    clients: CLIENTS,
    cases: CASES,
    periods: PERIODS,
    contributions: CONTRIBUTIONS,
    appointments: APPOINTMENTS,
    documents: DOCUMENTS
  };
}

function applyWorkspace(ws, source) {
  CLIENTS = ws.clients;
  CASES = ws.cases;
  PERIODS = ws.periods || {};
  CONTRIBUTIONS = ws.contributions || {};
  APPOINTMENTS = ws.appointments || [];
  DOCUMENTS = ws.documents || [];
  state.workspaceSource = source;
  state.caseId = CASES[0].id;
  state.caseTab = 'overview';
  state.reportCase = CASES[0].id;
  state.docId = DOCUMENTS.length ? DOCUMENTS[0].id : 1;
  state.view = 'home';
}

function saveWorkspace() {
  try { localStorage.setItem(WORKSPACE_KEY, JSON.stringify(workspacePayload())); } catch (e) {}
}

function loadSavedWorkspace() {
  let text = null;
  try { text = localStorage.getItem(WORKSPACE_KEY); } catch (e) { return false; }
  if (!text) return false;
  try {
    applyWorkspace(normalizeWorkspace(JSON.parse(text)), 'imported');
    return true;
  } catch (e) {
    try { localStorage.removeItem(WORKSPACE_KEY); } catch (err) {}
    return false;
  }
}

function restoreDemo() {
  const view = state.view;
  applyWorkspace(normalizeWorkspace(clone(DEMO)), 'demo');
  state.view = view;
  try { localStorage.removeItem(WORKSPACE_KEY); } catch (e) {}
  render();
  toast('Demonstração restaurada. Os dados importados foram removidos deste navegador.');
}

function downloadWorkspace() {
  const blob = new Blob([JSON.stringify(workspacePayload(), null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'previdenciar-workspace.json';
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  toast('Workspace exportado em JSON. Nenhum dado saiu deste navegador.');
}

function importWorkspaceText(text) {
  const ws = normalizeWorkspace(JSON.parse(text));
  applyWorkspace(ws, 'imported');
  saveWorkspace();
  render();
  toast('Workspace importado: ' + ws.clients.length + ' cliente' + (ws.clients.length === 1 ? '' : 's') + ' e ' + ws.cases.length + ' cálculo' + (ws.cases.length === 1 ? '' : 's') + '.');
}

function workspaceCounts() {
  const contributions = Object.keys(CONTRIBUTIONS).reduce((sum, key) => sum + (CONTRIBUTIONS[key] || []).length, 0);
  return { clients: CLIENTS.length, cases: CASES.length, contributions: contributions, appointments: APPOINTMENTS.length, documents: DOCUMENTS.length };
}
