function viewReports() {
  const c = caseById(state.reportCase);
  const options = CASES.map((x) => '<option value="' + x.id + '"' + (x.id === c.id ? ' selected' : '') + '>' + esc(x.name) + ' — ' + esc(clientById(x.client).name) + '</option>').join('');
  const rows = REPORT_TYPES.map((r, i) => '<label class="report-row"><input type="checkbox"' + (i < 3 ? ' checked' : '') + '><div><strong>' + esc(r.name) + '</strong><span>' + esc(r.desc) + '</span></div><span class="state state--quiet">PDF</span></label>').join('');
  return '<div class="reports-view">' + pageHead('Relatórios', 'Uma central para escolher o que entra no lote, com prévia e sem sobrescrever arquivo existente.') +
    '<div class="toolbar"><label class="checkline" for="reportCase">Cálculo</label><select class="select" id="reportCase" style="min-width:min(420px,100%)">' + options + '</select><span class="count">8 modelos</span></div>' +
    '<div class="case-grid"><div class="stack"><section class="folio"><div class="folio__head"><div><div class="folio__title">Documentos do lote</div><div class="folio__sub">Oito relatórios previstos; marque os que deseja gerar.</div></div><button type="button" class="btn btn--primary" data-report-batch>' + ico('download', 15) + 'Gerar lote</button></div><div class="folio__body">' + rows + '<div class="form-section" style="margin-top:12px"><div class="form-section__title">Opções de apresentação</div><label class="checkline" style="display:flex;margin:6px 0"><input type="checkbox" checked> Incluir anexo técnico e páginas de origem</label><label class="checkline" style="display:flex;margin:6px 0"><input type="checkbox" checked> Incluir conferência de qualidade</label><label class="checkline" style="display:flex;margin:6px 0"><input type="checkbox"> Incluir premissas de planejamento</label></div></div></section>' +
    '<div class="notice">' + ico('alert', 16) + '<p><strong>Demonstração.</strong> Nenhum PDF é gerado no navegador. A exportação real acontece no app desktop, pelo renderer Rust.</p></div></div>' +
    '<aside class="side-stack"><section class="folio"><div class="folio__head"><div class="folio__title">Prévia</div><span class="folio__sub">1 de 8</span></div><div class="folio__body"><div class="cover"><span class="cover__rule"></span><strong>Relatório completo</strong><span>' + esc(clientById(c.client).name) + '</span><span>' + esc(c.name) + '</span><div class="cover__foot">previdenciar.io<br>DIB ' + dateBR(c.dib) + ' · demonstração<br>Documento sem validade oficial</div></div><div class="form-actions"><button type="button" class="btn btn--quiet" data-toast="Prévia simulada; nenhum arquivo foi gravado.">Abrir prévia</button><button type="button" class="btn btn--quiet" data-print>Imprimir</button><button type="button" class="btn btn--primary" data-toast="Exportação simulada. Na versão desktop, o PDF é gravado na pasta escolhida.">Exportar</button></div></div></section></aside></div></div>';
}

function calendar() {
  const first = new Date(2026, 9, 1);
  const offset = first.getDay();
  const days = 31;
  const dows = ['dom', 'seg', 'ter', 'qua', 'qui', 'sex', 'sáb'];
  let html = dows.map((d) => '<div class="cal__dow">' + d + '</div>').join('');
  for (let i = 0; i < offset; i++) html += '<div class="cal__day cal__day--out"></div>';
  for (let d = 1; d <= days; d++) {
    const iso = '2026-10-' + String(d).padStart(2, '0');
    const apps = APPOINTMENTS.filter((a) => a.date === iso);
    const marks = apps.map((a) => '<span class="cal__mark" title="' + esc(a.title) + '">' + a.time + ' ' + esc(a.title) + '</span>').join('');
    html += '<div class="cal__day' + (d === 26 ? ' cal__day--today' : '') + '">' + d + marks + '</div>';
  }
  return html;
}

function viewAgenda() {
  const list = APPOINTMENTS.map((a) => '<div class="appt"><time>' + a.time + '</time><div><strong>' + esc(a.title) + '</strong><span>' + esc(a.who) + ' — ' + dateBR(a.date) + '</span></div></div>').join('');
  return pageHead('Agenda', 'Compromissos locais, sem convites, notificações ou sincronização externa.', '<button type="button" class="btn btn--primary" data-toast="Novo compromisso simulado. A agenda real fica na base local.">' + ico('plus', 15) + 'Novo compromisso</button>') +
    '<div class="agenda"><section class="folio"><div class="folio__head"><div><div class="folio__title">Outubro de 2026</div><div class="folio__sub">Hoje, 26 de outubro.</div></div><span class="count">6 compromissos</span></div><div class="folio__body"><div class="cal">' + calendar() + '</div></div></section>' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Próximos</div></div><div class="folio__body">' + list + '<p class="form-note">Datas e anotações do escritório; nada é enviado a terceiros.</p></div></section></div>';
}

function viewSettings() {
  return pageHead('Configurações', 'Identidade do escritório, aparência, base local e privacidade. Nada aqui sai da máquina.') +
    '<div class="settings-grid"><div class="stack">' +
    '<section class="folio"><div class="folio__head"><div><div class="folio__title">Dados do workspace</div><div class="folio__sub">JSON versão 1 · importação e exportação locais</div></div>' + stateTag(state.workspaceSource === 'imported' ? 'Importado' : 'Demonstração', state.workspaceSource === 'imported' ? 'ok' : 'quiet') + '</div><div class="folio__body"><dl class="ledger-list" style="margin:0">' +
    '<div class="ledger-row"><dt>Clientes</dt><dd class="mono">' + workspaceCounts().clients + '</dd></div>' +
    '<div class="ledger-row"><dt>Casos</dt><dd class="mono">' + workspaceCounts().cases + '</dd></div>' +
    '<div class="ledger-row"><dt>Contribuições</dt><dd class="mono">' + workspaceCounts().contributions + '</dd></div>' +
    '<div class="ledger-row"><dt>Compromissos</dt><dd class="mono">' + workspaceCounts().appointments + '</dd></div>' +
    '<div class="ledger-row"><dt>Documentos</dt><dd class="mono">' + workspaceCounts().documents + '</dd></div>' +
    '</dl><div class="form-actions"><button type="button" class="btn btn--quiet" data-export>Baixar dados</button><button type="button" class="btn btn--quiet" data-import-trigger>Importar JSON</button><a class="btn btn--ghost" href="workspace.example.json" download>Baixar exemplo</a><button type="button" class="btn btn--quiet" data-restore-demo' + (state.workspaceSource === 'demo' ? ' disabled' : '') + '>Restaurar demonstração</button><input id="importFile" type="file" accept=".json,application/json" hidden></div><p class="form-note">O JSON não é cifrado. Handele o arquivo como dado de cliente e importe apenas em uma máquina confiável. O motor não é executado no navegador.</p></div></section>' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Identidade do escritório</div></div><div class="folio__body"><div class="form-grid">' +
    '<div class="field"><label for="s-name">Nome institucional</label><input class="input" id="s-name" value="Rossetto Previdenciário"></div>' +
    '<div class="field"><label for="s-email">E-mail</label><input class="input" id="s-email" value="contato@exemplo.com"></div>' +
    '<div class="field field--wide"><label for="s-foot">Rodapé dos relatórios</label><textarea class="textarea" id="s-foot">Documento produzido com apoio de ferramenta experimental. Revisão profissional obrigatória.</textarea></div>' +
    '</div><div class="form-actions"><button type="button" class="btn btn--primary" data-toast="Configurações salvas na demonstração.">Salvar configurações</button></div></div></section>' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Armazenamento local</div></div><div class="folio__body"><div class="toggleline"><div><strong>Workspace no navegador</strong><span>Importação e persistência em localStorage; o JSON não é cifrado.</span></div>' + stateTag('Local', 'ok') + '</div><div class="toggleline"><div><strong>Exportar dados</strong><span>Baixe o workspace atual em JSON; nada é enviado a servidor.</span></div><button type="button" class="btn btn--quiet" data-export>Baixar dados</button></div></div></section>' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Atualizações</div></div><div class="folio__body"><div class="toggleline"><div><strong>Versão 0.10.5-web</strong><span>Mock navegável; a versão desktop verifica a release oficial.</span></div><button type="button" class="btn btn--quiet" data-toast="Verificação simulada. Nenhuma atualização foi baixada.">Verificar</button></div></div></section>' +
    '</div><div class="stack">' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Aparência</div></div><div class="folio__body"><div class="toggleline"><div><strong>Tema escuro</strong><span>Papel quente no claro; couro e tinta no escuro.</span></div><button type="button" class="switch" role="switch" aria-checked="' + (state.theme === 'dark') + '" aria-label="Tema escuro" data-toggle-theme></button></div><div class="toggleline"><div><strong>Reduzir animações</strong><span>Desliga a entrada das páginas e as transições.</span></div><button type="button" class="switch" role="switch" aria-checked="' + (state.motion === 'reduced') + '" aria-label="Reduzir animações" data-toggle-motion></button></div></div></section>' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Privacidade</div></div><div class="folio__body"><p class="form-note" style="margin:0">Importação, leitura e exportação ocorrem neste navegador. Cálculo e PDF não são executados. Nenhum dado é enviado a servidor.</p><div class="notice" style="margin-top:12px">' + ico('lock', 16) + '<p><strong>Identificação oculta não garante anonimização.</strong> Datas e valores ainda permitem correlação.</p></div></div></section>' +
    '<section class="folio"><div class="folio__head"><div class="folio__title">Sobre esta demonstração</div></div><div class="folio__body"><dl class="ledger-list" style="margin:0"><div class="ledger-row"><dt>Versão web</dt><dd class="mono">0.10.5-web</dd></div><div class="ledger-row"><dt>Dados</dt><dd>' + (state.workspaceSource === 'imported' ? 'Importados' : 'Fictícios') + '</dd></div><div class="ledger-row"><dt>Motor</dt><dd>Não executado</dd></div></dl><p class="form-note">A versão web é um mock navegável. Com um workspace importado, ela lê os dados sem executar o motor. A versão desktop usa React, Tauri e Rust.</p></div></section>' +
    '</div></div>';
}

function viewCoverage() {
  const rows = COVERAGE.map((r) => '<tr><td><strong>' + esc(r.surface) + '</strong></td><td>' + esc(r.observed) + '</td><td>' + esc(r.inferred) + '</td><td>' + stateTag(r.coverage, r.tone) + '</td></tr>').join('');
  const path = PDF_PATH.map((p, i) => '<div class="ledger-row"><dt><span class="mono muted">0' + (i + 1) + '</span> ' + esc(p) + '</dt><dd class="mono" style="color:var(--jade)">observado</dd></div>').join('');
  return pageHead('REA e validação', 'Ferramentas reais, evidências locais e a distinção entre o que foi observado e o que foi inferido.') +
    '<div class="brief"><span>REA <strong>6.5.0</strong></span><span>207 ferramentas</span><span>1.902 funções</span><span>1.196 strings</span></div>' +
    '<div class="case-grid"><div class="stack"><section class="folio"><div class="folio__head"><div><div class="folio__title">Cobertura observada</div><div class="folio__sub">O que a investigação alcançou e o que permanece opaco.</div></div></div><div class="folio__body folio__body--flush"><div class="table-wrap"><table class="table"><thead><tr><th>Superfície</th><th>Observado</th><th>Inferido</th><th>Cobertura</th></tr></thead><tbody>' + rows + '</tbody></table></div></div></section>' +
    '<div class="notice">' + ico('alert', 16) + '<p><strong>A existência de uma tela não comprova equivalência funcional.</strong> O relatório de testes registra separadamente fluxo de interface, operação real do Rust e revisão normativa pendente.</p></div></div>' +
    '<aside class="side-stack"><section class="folio"><div class="folio__head"><div class="folio__title">Caminho do PDF</div><span class="stamp">7 funções</span></div><div class="folio__body"><div class="ledger-list">' + path + '</div><p class="form-note">Wrapper, stream, configuração, saída, objetos, laço e render. As fórmulas previdenciárias não foram recuperadas.</p></div></section><div class="notice notice--bad">' + ico('lock', 16) + '<p><strong>INFOEXE.</strong> 13.587.224 bytes permanecem opacos. Nenhuma conclusão foi inferida do que não foi lido.</p></div></aside></div>';
}
