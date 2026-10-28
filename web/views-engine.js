function endOfMonth(ym) {
  const parts = String(ym || '').split('-').map(Number);
  if (parts.length < 2 || !parts[0] || !parts[1]) return ym;
  const date = new Date(parts[0], parts[1], 0);
  return date.getFullYear() + '-' + String(date.getMonth() + 1).padStart(2, '0') + '-' + String(date.getDate()).padStart(2, '0');
}

function engineKind(kind) {
  const text = String(kind || '').toLowerCase();
  if (text.indexOf('aut') >= 0 || text.indexOf('individual') >= 0) return 'contribuinte_individual';
  if (text.indexOf('especial') >= 0) return 'especial';
  if (text.indexOf('dom') >= 0) return 'empregado_domestico';
  return 'empregado';
}

function engineMoney(value) {
  const raw = String(value == null ? '' : value).trim();
  if (!raw || /[•—]/.test(raw)) return null;
  let text = raw.replace(/[^\d.,-]/g, '');
  if (text.indexOf(',') >= 0) text = text.replace(/\./g, '').replace(',', '.');
  return /^\d+(\.\d{1,2})?$/.test(text) ? text : null;
}

function engineInputFromCase(c) {
  if (c.kind !== 'RGPS') throw new Error('O núcleo wasm cobre a espécie 42 (RGPS). O caso atual é ' + c.kind + '; use o exemplo sintético ou o app desktop.');
  const cl = clientById(c.client);
  const periods = (PERIODS[c.id] || []).length
    ? PERIODS[c.id]
    : [{ employer: 'Vínculo principal', kind: 'Empregado', start: c.start, end: c.end, source: 'Caso' }];
  const vinculos = periods.map((p) => ({
    inicio: p.start + '-01',
    fim: endOfMonth(p.end),
    tipo: engineKind(p.kind),
    empresa: p.employer,
    fator: 1
  }));
  const competencias = contribsFor(c)
    .filter((row) => row.status !== 'excluido' && row.status !== 'pendente')
    .map((row) => ({ competencia: row.comp, salario: engineMoney(row.value), confirmada: row.status !== 'pendente' }))
    .filter((row) => row.salario);
  if (!competencias.length) throw new Error('O caso não tem competências válidas para o motor. Importe um workspace com salários confirmados.');
  return {
    nome: cl.name,
    especie: '42',
    segurado: { sexo: String(cl.sex || '').toLowerCase(), dataNascimento: cl.birth || null, deficiencia: 'nao', especie: '42' },
    dataReferencia: c.dib,
    vinculos: vinculos,
    competencias: competencias,
    opcoes: { corrigirSalarios: false, descarte: false, divisorMinimo: false, aplicarTetoVigenteDer: true },
    revisao: { confirmada: true, evidencia: 'Workspace do navegador', politica: 'rgps-core-observational-v1' }
  };
}

function engineRaw(out) {
  return '<details class="engine-raw"><summary>JSON completo do motor</summary><pre class="json">' + esc(JSON.stringify(out, null, 2)) + '</pre></details>';
}

function engineResultHtml(out, label) {
  if (!out || out.ok !== true) {
    return '<div class="notice notice--bad">' + ico('alert', 16) + '<p><strong>O motor recusou a entrada.</strong> ' + esc((out && out.error) || 'Erro desconhecido.') + '</p></div>' + engineRaw(out) + '<p class="form-note">Entrada: ' + esc(label) + '. O bloqueio é parte do contrato; nenhum campo ausente foi inventado.</p>';
  }
  const data = out.data || {};
  const resumo = data.resumo || {};
  const rows = [
    ['Competências', String(resumo.competencias == null ? '—' : resumo.competencias)],
    ['Carência', resumo.carencia_meses == null ? '—' : resumo.carencia_meses + ' meses'],
    ['Salário de benefício', resumo.salario_beneficio == null ? '—' : brl(Number(resumo.salario_beneficio))],
    ['Coeficiente', resumo.coeficiente == null ? '—' : String(resumo.coeficiente)],
    ['RMI', resumo.rmi == null ? '—' : brl(Number(resumo.rmi))],
    ['Limite aplicado', resumo.limite_aplicado || '—']
  ].map((row) => '<div class="ledger-row"><dt>' + row[0] + '</dt><dd class="mono">' + esc(row[1]) + '</dd></div>').join('');
  const warnings = Array.isArray(data.avisos) && data.avisos.length
    ? '<div class="notice">' + ico('alert', 16) + '<div><strong>Avisos do motor</strong><ul class="engine-warnings">' + data.avisos.map((w) => '<li>' + esc(w) + '</li>').join('') + '</ul></div></div>'
    : '';
  return '<div class="ledger-list">' + rows + '</div>' + warnings + '<p class="form-note">Entrada: ' + esc(label) + '. Resultado experimental do rgps-core; não certifica direito e não substitui revisão profissional.</p>' + engineRaw(out);
}

async function initEnginePage() {
  const status = $('#engineStatus');
  const version = $('#engineVersion');
  if (!status || !window.PioEngine) return;
  try {
    const info = await window.PioEngine.version();
    if (version) version.textContent = info.version || '—';
    status.textContent = 'Pronto';
    status.className = 'state state--ok';
  } catch (error) {
    if (version) version.textContent = '—';
    status.textContent = 'Indisponível';
    status.className = 'state state--bad';
    const body = $('#engineResult');
    const wrap = $('#engineResultWrap');
    if (body && wrap) {
      wrap.hidden = false;
      body.innerHTML = '<div class="notice notice--bad">' + ico('alert', 16) + '<p><strong>O WebAssembly não carregou.</strong> ' + esc(error.message || String(error)) + ' Abra via servidor local ou verifique engine.data.js.</p></div>';
    }
  }
}

async function runEngine(mode) {
  const status = $('#engineStatus');
  const wrap = $('#engineResultWrap');
  const body = $('#engineResult');
  const badge = $('#engineResultStatus');
  if (!body || !wrap) return;
  const buttons = $$('[data-engine-example],[data-engine-case]');
  buttons.forEach((button) => { button.disabled = true; });
  status.textContent = 'Executando';
  status.className = 'state state--warn';
  wrap.hidden = false;
  body.innerHTML = '<p class="muted">Executando rgps-core no WebAssembly…</p>';
  if (badge) badge.innerHTML = '';
  try {
    let input;
    let label;
    if (mode === 'example') {
      input = window.PIO_ENGINE_EXAMPLE;
      label = 'fixture sintético de 300 competências';
      if (!input) throw new Error('Exemplo embutido ausente.');
    } else {
      const current = caseById(state.caseId);
      input = engineInputFromCase(current);
      label = 'caso ' + current.name + ' (' + input.competencias.length + ' competências)';
    }
    const out = await window.PioEngine.calculate(input);
    body.innerHTML = engineResultHtml(out, label);
    if (badge) badge.innerHTML = stateTag(out.ok ? 'Calculado' : 'Bloqueado', out.ok ? 'ok' : 'bad');
    status.textContent = 'Pronto';
    status.className = 'state state--ok';
  } catch (error) {
    body.innerHTML = '<div class="notice notice--bad">' + ico('alert', 16) + '<p><strong>Falha ao executar.</strong> ' + esc(error.message || String(error)) + '</p></div>';
    if (badge) badge.innerHTML = stateTag('Erro', 'bad');
    status.textContent = 'Erro';
    status.className = 'state state--bad';
  } finally {
    buttons.forEach((button) => { button.disabled = false; });
  }
}

function viewEngine() {
  return pageHead('Motor local (WASM)', 'O rgps-core compilado para WebAssembly. Roda neste navegador, sem servidor, sem Tauri e sem enviar dados.') +
    '<div class="brief"><span>Motor <strong>rgps-core</strong></span><span>Versão <strong id="engineVersion">carregando</strong></span><span>Onde roda <strong>neste navegador</strong></span></div>' +
    '<div class="case-grid"><div class="stack">' +
    '<section class="folio"><div class="folio__head"><div><div class="folio__title">Executar o motor</div><div class="folio__sub">Duas entradas: fixture sintético de 300 competências ou os dados do caso atual.</div></div><span class="state state--quiet" id="engineStatus">Carregando</span></div><div class="folio__body">' +
    '<p style="margin:0;color:var(--ink-2);font-size:13px">A entrada é montada em JSON; o motor devolve tempo, carência, PBC, coeficiente, RMI, transições e avisos. Se faltar um campo, ele bloqueia em vez de inventar.</p>' +
    '<div class="form-actions"><button type="button" class="btn btn--primary" data-engine-example>Executar exemplo sintético</button><button type="button" class="btn btn--quiet" data-engine-case>Executar dados do caso</button></div>' +
    '<p class="form-note">O exemplo é o fixture do repositório, com 300 competências. O botão de caso usa o workspace atual e o módulo RGPS; se o caso não tiver salários válidos ou não for RGPS, o motor explica o bloqueio. PcD, pensão e outros módulos ficam no desktop.</p></div></section>' +
    '<section class="folio" id="engineResultWrap" hidden><div class="folio__head"><div><div class="folio__title">Resultado</div><div class="folio__sub">Saída bruta do rgps-core, sem pós-processamento de RMI.</div></div><span id="engineResultStatus"></span></div><div class="folio__body" id="engineResult"></div></section>' +
    '<div class="notice">' + ico('alert', 16) + '<p><strong>Experimental.</strong> O motor organiza a memória e mostra o cálculo; não certifica direito, não substitui revisão profissional e não cobre todos os regimes.</p></div>' +
    '</div><aside class="side-stack">' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">O que roda aqui</div></div><div class="folio__body"><dl class="ledger-list" style="margin:0">' +
    '<div class="ledger-row"><dt>Motor</dt><dd class="mono">rgps-core 0.1.0</dd></div>' +
    '<div class="ledger-row"><dt>Alvo</dt><dd class="mono">wasm32</dd></div>' +
    '<div class="ledger-row"><dt>Entrada</dt><dd>JSON</dd></div>' +
    '<div class="ledger-row"><dt>Rede</dt><dd>nenhuma</dd></div>' +
    '<div class="ledger-row"><dt>CNIS / PDF</dt><dd>fora do wasm</dd></div>' +
    '</dl><p class="form-note">O adaptador previus-rgps, o CNIS, o workspace cifrado e o renderer de PDF continuam no desktop.</p></div></section>' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Reprodutibilidade</div></div><div class="folio__body"><div class="form-actions" style="justify-content:flex-start"><a class="btn btn--quiet" href="engine.wasm" download>Baixar wasm</a><a class="btn btn--ghost" href="engine.example.json" download>Baixar exemplo</a></div><p class="form-note">Regere com <span class="mono">scripts/build-web-engine.ps1</span> e <span class="mono">cargo build -p previus-wasm --target wasm32-unknown-unknown --release</span>.</p></div></section>' +
    '<div class="notice">' + ico('lock', 16) + '<p><strong>Nenhum dado sai do navegador.</strong> A entrada e a saída ficam na memória da página; o wasm não faz requisições.</p></div>' +
    '</aside></div>';
}
