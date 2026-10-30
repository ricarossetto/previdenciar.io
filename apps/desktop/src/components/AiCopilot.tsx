import { useEffect, useMemo, useRef, useState } from 'react';
import { api, errorMessage } from '../api';
import type { AiChatMessage, AiDataStatus, AiEvent, AiKeyStatus, AiProviderKind, AiProviderPreset, AiSource, AiTestResult, AiUsage } from '../api';
import { caseSummary } from '../aiCase';
import type { Workspace } from '../model';
import { Badge, Button, Icon, Notice, Panel } from './ui';

interface StoredConfig { providerId: string; kind: AiProviderKind; baseUrl: string; model: string }
interface ToolActivity { name: string; status: 'running' | 'done' | 'error'; summary: string; sources: AiSource[] }
interface UiMessage extends AiChatMessage { id: string; tools: ToolActivity[]; sources: AiSource[]; usage: AiUsage | null; finish: string | null }
interface NoticeState { message: string; error: boolean }
interface CopilotCache { messages: UiMessage[]; input: string; caseId: number | null; attachCase: boolean; toolsEnabled: boolean }

const CONFIG_KEY = 'previdenciar.ai.config.v1';
let copilotCache: CopilotCache | null = null;
export function resetAiCopilotCache() { copilotCache = null; }
const DEFAULT_MAX_TOKENS = 1200;
const SUGGESTIONS = [
  'Quais documentos devo conferir antes de revisar uma RMI?',
  'Explique a diferença entre carência e tempo de contribuição.',
  'Pesquise na base local normas sobre aposentadoria por incapacidade.',
  'Quais precedentes locais tratam de coisa julgada previdenciária?',
];

function readConfig(): StoredConfig | null {
  try {
    const value = JSON.parse(localStorage.getItem(CONFIG_KEY) ?? 'null') as StoredConfig | null;
    if (!value || typeof value.providerId !== 'string') return null;
    return value;
  } catch { return null; }
}
function newId(prefix: string): string {
  return `${prefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 9)}`;
}
function toolLabel(name: string): string {
  return ({
    buscar_habilidades: 'Habilidades',
    ler_habilidade: 'Ler habilidade',
    buscar_legislacao: 'Legislação',
    ler_legislacao: 'Ler norma',
    buscar_natjus: 'NatJus',
    ler_natjus: 'Ler NatJus',
    buscar_jurisprudencia: 'Jurisprudência',
    ler_jurisprudencia: 'Ler ementa',
  } as Record<string, string>)[name] ?? name;
}
function applyEvent(messages: UiMessage[], event: AiEvent): UiMessage[] {
  const index = [...messages].reverse().findIndex((message) => message.role === 'assistant');
  if (index < 0) return messages;
  const target = messages.length - 1 - index;
  const copy = [...messages];
  const message = { ...copy[target], tools: [...copy[target].tools], sources: [...copy[target].sources] };
  if (event.type === 'delta') message.content += event.text;
  if (event.type === 'tool') {
    const running = message.tools.findIndex((tool) => tool.name === event.name && tool.status === 'running');
    const activity: ToolActivity = { name: event.name, status: event.status, summary: event.summary, sources: event.sources };
    if (running >= 0) message.tools[running] = activity;
    else message.tools.push(activity);
    for (const source of event.sources) {
      if (!message.sources.some((item) => item.label === source.label && item.title === source.title)) message.sources.push(source);
    }
  }
  if (event.type === 'done') { message.usage = event.usage; message.finish = event.finish; }
  copy[target] = message;
  return copy;
}

export function AiCopilot({ notify, workspace }: { notify: (message: string, error?: boolean) => void; workspace: Workspace }) {
  const [catalog, setCatalog] = useState<AiProviderPreset[]>([]);
  const [config, setConfig] = useState<StoredConfig>({ providerId: '', kind: 'openai', baseUrl: '', model: '' });
  const [keys, setKeys] = useState<Record<string, boolean>>({});
  const [dataStatus, setDataStatus] = useState<AiDataStatus | null>(null);
  const [apiKey, setApiKey] = useState('');
  const [consent, setConsent] = useState(false);
  const [toolsEnabled, setToolsEnabled] = useState(() => copilotCache?.toolsEnabled ?? true);
  const [messages, setMessages] = useState<UiMessage[]>(() => {
    const cached = copilotCache?.messages ?? [];
    const last = cached[cached.length - 1];
    return last?.role === 'assistant' && last.finish === null ? cached.slice(0, -1) : cached;
  });
  const [input, setInput] = useState(() => copilotCache?.input ?? '');
  const [busy, setBusy] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const [savingKey, setSavingKey] = useState(false);
  const [testing, setTesting] = useState(false);
  const [discovering, setDiscovering] = useState(false);
  const [importing, setImporting] = useState(false);
  const [includePdf, setIncludePdf] = useState(true);
  const [caseId, setCaseId] = useState<number | null>(() => copilotCache?.caseId ?? null);
  const [attachCase, setAttachCase] = useState(() => copilotCache?.attachCase ?? false);
  const [testResult, setTestResult] = useState<AiTestResult | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [notice, setNotice] = useState<NoticeState | null>(null);
  const activeRequest = useRef<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const selected = useMemo(() => catalog.find((item) => item.id === config.providerId) ?? null, [catalog, config.providerId]);
  const hasKey = Boolean(keys[config.providerId]);
  const keyReady = hasKey || selected?.keyRequired === false;
  const canSend = !busy && Boolean(input.trim()) && consent && keyReady && Boolean(config.model.trim()) && Boolean(config.baseUrl.trim());
  const cases = useMemo(() => workspace.calculations.filter((item) => !item.archived), [workspace.calculations]);
  const selectedCase = useMemo(() => cases.find((item) => item.id === caseId) ?? null, [cases, caseId]);
  const caseContext = useMemo(() => selectedCase && attachCase ? caseSummary(selectedCase) : null, [selectedCase, attachCase]);

  useEffect(() => {
    let alive = true;
    void Promise.all([api.aiCatalog(), api.aiKeyStatus(), api.aiDataStatus()]).then(([providers, statuses, status]) => {
      if (!alive) return;
      setCatalog(providers);
      setKeys(Object.fromEntries(statuses.map((item: AiKeyStatus) => [item.providerId, item.hasKey])));
      setDataStatus(status);
      const saved = readConfig();
      const fallback = providers.find((item) => item.id === saved?.providerId) ?? providers[0] ?? null;
      if (fallback) {
        setConfig({
          providerId: fallback.id,
          kind: saved?.providerId === fallback.id ? saved.kind : fallback.kind,
          baseUrl: saved?.providerId === fallback.id && saved.baseUrl ? saved.baseUrl : fallback.baseUrl,
          model: saved?.providerId === fallback.id && saved.model ? saved.model : fallback.defaultModel,
        });
      }
    }).catch((error) => { if (alive) setNotice({ message: errorMessage(error), error: true }); });
    return () => { alive = false; };
  }, []);

  useEffect(() => {
    if (!config.providerId) return;
    try { localStorage.setItem(CONFIG_KEY, JSON.stringify(config)); } catch { /* config não secreta; falha não bloqueia */ }
  }, [config]);

  useEffect(() => {
    copilotCache = { messages, input, caseId, attachCase, toolsEnabled };
  }, [messages, input, caseId, attachCase, toolsEnabled]);

  useEffect(() => () => {
    const requestId = activeRequest.current;
    if (requestId) void api.aiCancel(requestId).catch(() => {});
  }, []);

  useEffect(() => {
    const node = scrollRef.current;
    if (node) node.scrollTop = node.scrollHeight;
  }, [messages, streaming]);

  useEffect(() => {
    if (caseId !== null && !cases.some((item) => item.id === caseId)) {
      setCaseId(null);
      setAttachCase(false);
    }
  }, [cases, caseId]);

  function chooseProvider(providerId: string) {
    const provider = catalog.find((item) => item.id === providerId);
    if (!provider) return;
    setConfig({ providerId, kind: provider.kind, baseUrl: provider.baseUrl, model: provider.defaultModel });
    setApiKey('');
    setTestResult(null);
    setModels([]);
  }
  function updateConfig(patch: Partial<StoredConfig>) { setConfig((current) => ({ ...current, ...patch })); }
  function useSuggestion(text: string) { setInput(text); inputRef.current?.focus(); }
  function clearChat() {
    if (busy) return;
    setMessages([]);
    setNotice(null);
  }
  async function saveKey() {
    if (!config.providerId) return;
    if (!apiKey.trim()) { setNotice({ message: 'Cole a chave de API antes de salvar.', error: true }); return; }
    setSavingKey(true);
    try {
      await api.aiSaveKey(config.providerId, apiKey);
      setKeys((current) => ({ ...current, [config.providerId]: true }));
      setApiKey('');
      setNotice({ message: 'Chave salva no cofre DPAPI deste usuário Windows.', error: false });
      notify('Chave de IA salva localmente com DPAPI.');
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
    finally { setSavingKey(false); }
  }
  async function deleteKey() {
    if (!config.providerId) return;
    setSavingKey(true);
    try {
      await api.aiDeleteKey(config.providerId);
      setKeys((current) => ({ ...current, [config.providerId]: false }));
      setTestResult(null);
      setNotice({ message: 'Chave removida do cofre local.', error: false });
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
    finally { setSavingKey(false); }
  }
  async function testProvider() {
    if (!config.providerId) return;
    setTesting(true);
    setTestResult(null);
    try {
      const result = await api.aiTest(config.providerId, config.kind, config.baseUrl, config.model);
      setTestResult(result);
      setNotice({ message: `Conexão confirmada em ${result.latencyMs} ms.`, error: false });
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
    finally { setTesting(false); }
  }
  async function discoverModels() {
    if (!config.providerId) return;
    setDiscovering(true);
    try {
      const found = await api.aiModels(config.providerId, config.kind, config.baseUrl);
      setModels(found);
      if (found.length && !found.includes(config.model)) updateConfig({ model: found[0] });
      setNotice({ message: `${found.length} modelos encontrados no provedor.`, error: false });
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
    finally { setDiscovering(false); }
  }
  async function importData() {
    setImporting(true);
    try {
      const summary = await api.aiImportData(includePdf);
      if (!summary) { setNotice({ message: 'Nenhuma pasta foi selecionada.', error: false }); return; }
      setDataStatus(await api.aiDataStatus());
      setNotice({ message: `Base importada: ${summary.skills} habilidades, ${summary.legislation} normas, ${summary.natjus} NatJus (${summary.natjusText} com texto extraído${summary.natjusTextFailures ? `; ${summary.natjusTextFailures} sem texto` : ''}) e ${summary.jurisprudence} ementas.`, error: false });
      notify('Base local do Copiloto atualizada.');
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
    finally { setImporting(false); }
  }
  async function cancel() {
    const requestId = activeRequest.current;
    if (!requestId) return;
    try { await api.aiCancel(requestId); setNotice({ message: 'Cancelamento solicitado; o provedor pode levar alguns segundos para encerrar.', error: false }); }
    catch (error) { setNotice({ message: errorMessage(error), error: true }); }
  }
  async function send(text?: string) {
    if (busy) return;
    if (!consent) { setNotice({ message: 'Marque a autorização de envio ao provedor antes de usar o Copiloto.', error: true }); return; }
    if (!keyReady) { setNotice({ message: 'Salve a chave de API deste provedor ou escolha um endpoint local sem chave.', error: true }); return; }
    const content = (text ?? input).trim();
    if (!content) return;
    if (!config.model.trim() || !config.baseUrl.trim()) { setNotice({ message: 'Confira a URL base e o modelo antes de enviar.', error: true }); return; }
    const user: UiMessage = { id: newId('u'), role: 'user', content, tools: [], sources: [], usage: null, finish: null };
    const assistant: UiMessage = { id: newId('a'), role: 'assistant', content: '', tools: [], sources: [], usage: null, finish: null };
    const history: AiChatMessage[] = [...messages.filter((message) => message.content.trim()).map(({ role, content: body }) => ({ role, content: body })), { role: 'user', content }];
    const requestId = newId('req');
    activeRequest.current = requestId;
    setMessages((current) => [...current, user, assistant]);
    setInput('');
    setBusy(true);
    setStreaming(true);
    setNotice(null);
    try {
      await api.aiChat({ requestId, providerId: config.providerId, kind: config.kind, baseUrl: config.baseUrl, model: config.model, messages: history, consent, toolsEnabled, maxTokens: DEFAULT_MAX_TOKENS, caseContext }, (event) => {
        setMessages((current) => applyEvent(current, event));
      });
    } catch (error) {
      const message = errorMessage(error);
      const cancelled = /cancelad/i.test(message);
      setNotice({ message, error: !cancelled });
      setMessages((current) => {
        const copy = [...current];
        const last = copy.at(-1);
        if (last?.role === 'assistant' && !last.content && last.tools.length === 0) copy.pop();
        return copy;
      });
    } finally {
      activeRequest.current = null;
      setBusy(false);
      setStreaming(false);
    }
  }

  const dataReady = dataStatus?.ready === true;
  const command = 'powershell -ExecutionPolicy Bypass -File scripts/import-base-conhecimento.ps1';
  return <div className="ai-page">
    <div className="ai-banner">
      <div className="ai-banner-mark"><Icon name="spark" size={22}/></div>
      <div>
        <strong>Copiloto opcional com provedor externo</strong>
        <p>O envio começa desativado. Sua pergunta e o histórico desta conversa só saem da máquina depois que você autorizar. Nenhum CNIS, cliente ou cálculo é enviado automaticamente.</p>
      </div>
      <Badge tone="review">Nuvem opcional</Badge>
    </div>

    <div className="ai-layout">
      <section className="ai-chat-shell" aria-label="Conversa com o Copiloto">
        <header className="ai-chat-head">
          <div>
            <span className="eyebrow">Apoio à pesquisa</span>
            <strong>Pergunte com fonte, revise antes de usar</strong>
          </div>
          <div className="ai-chat-head-actions">
            {caseContext && <Badge tone="review">Caso anexado</Badge>}
            <Badge tone={consent ? 'good' : 'neutral'}>{consent ? 'Envio autorizado' : 'Envio bloqueado'}</Badge>
            <Button onClick={clearChat} disabled={busy || messages.length === 0}>Limpar</Button>
          </div>
        </header>

        <div className="ai-chat-scroll" ref={scrollRef} aria-live="polite">
          {messages.length === 0 ? <div className="ai-empty">
            <span className="ai-empty-icon"><Icon name="spark" size={28}/></span>
            <h2>Um copiloto para conferir, não para decidir</h2>
            <p>Use para localizar habilidades, normas, pareceres NatJus e ementas na base local, ou para organizar uma dúvida jurídica. Cálculos e conclusões continuam no motor e na sua revisão.</p>
            <div className="ai-suggestions">
              {SUGGESTIONS.map((suggestion) => <button key={suggestion} type="button" className="ai-suggestion" onClick={() => useSuggestion(suggestion)}>{suggestion}<Icon name="arrow" size={15}/></button>)}
            </div>
            {!dataReady && <Notice tone="error">A base local ainda não foi importada. O Copiloto pode conversar, mas as ferramentas de habilidades, legislação, NatJus e jurisprudência não terão conteúdo.</Notice>}
          </div> : messages.map((message) => <article key={message.id} className={`ai-message ai-message--${message.role}`}>
            <header>
              <span>{message.role === 'user' ? 'Você' : 'Copiloto'}</span>
              {message.finish === 'length' && <Badge tone="review">Resposta truncada</Badge>}
              {message.usage && <small>{message.usage.inputTokens ?? '?'} entrada · {message.usage.outputTokens ?? '?'} saída</small>}
            </header>
            <div className="ai-message-text">{message.content || (message.role === 'assistant' && streaming ? <span className="ai-typing">Consultando…</span> : null)}</div>
            {message.tools.length > 0 && <div className="ai-tool-list" aria-label="Ferramentas consultadas">
              {message.tools.map((tool, index) => <div key={`${tool.name}-${index}`} className={`ai-tool ai-tool--${tool.status}`}>
                <Icon name="tool" size={14}/>
                <strong>{toolLabel(tool.name)}</strong>
                <span>{tool.summary}</span>
              </div>)}
            </div>}
            {message.sources.length > 0 && <div className="ai-source-list" aria-label="Fontes locais">
              <span className="ai-source-title">Fontes consultadas</span>
              {message.sources.map((source) => <div className="ai-source" key={`${source.label}-${source.title}`}>
                <span className="ai-ref">{source.label}</span>
                <span>{source.title}</span>
                {source.url && <button type="button" className="link-button" onClick={() => void navigator.clipboard?.writeText(source.url!).then(() => notify('Link copiado.')).catch(() => notify('Não foi possível copiar o link.', true))}>copiar link</button>}
              </div>)}
            </div>}
          </article>)}
        </div>

        {notice && <div className={`ai-notice ${notice.error ? 'is-error' : ''}`} role={notice.error ? 'alert' : 'status'}>
          <Icon name={notice.error ? 'shield' : 'check'} size={17}/><span>{notice.message}</span>
          <button type="button" className="iconbtn" aria-label="Fechar aviso" onClick={() => setNotice(null)}><Icon name="close" size={14}/></button>
        </div>}

        <div className="ai-composer">
          <label className="ai-input-wrap">
            <span className="sr-only">Pergunta para o Copiloto</span>
            <textarea ref={inputRef} value={input} onChange={(event) => setInput(event.target.value)} onKeyDown={(event) => {
              if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); void send(); }
            }} placeholder="Pergunte sobre uma tese, peça uma busca na base local ou peça ajuda para organizar a revisão…" rows={3} disabled={busy}/>
          </label>
          <div className="ai-composer-bar">
            <label className="checkline"><input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)}/>Permitir enviar esta conversa ao provedor</label>
            <div className="ai-composer-actions">
              {busy ? <Button onClick={() => void cancel()}>Cancelar</Button> : null}
              <Button primary disabled={!canSend} onClick={() => void send()}>{busy ? (streaming ? 'Gerando…' : 'Aguardando…') : 'Enviar'}<Icon name="arrow" size={16}/></Button>
            </div>
          </div>
          <p className="ai-disclaimer">Resposta assistida por IA. Não é parecer, cálculo ou validação jurídica. Confira as fontes oficiais e os dados do caso antes de usar.</p>
        </div>
      </section>

      <aside className="ai-side">
        <Panel title="Provedor" subtitle="A chave fica no cofre DPAPI local e não volta para a interface." actions={<Badge tone={hasKey ? 'good' : selected?.keyRequired === false ? 'neutral' : 'review'}>{hasKey ? 'Chave salva' : selected?.keyRequired === false ? 'Chave opcional' : 'Sem chave'}</Badge>}>
          <div className="ai-form">
            <label className="field"><span>Provedor</span><select className="input" value={config.providerId} onChange={(event) => chooseProvider(event.target.value)} disabled={busy}>
              {catalog.map((provider) => <option key={provider.id} value={provider.id}>{provider.name}</option>)}
            </select></label>
            {config.providerId === 'custom' && <label className="field"><span>Compatibilidade</span><select className="input" value={config.kind} onChange={(event) => updateConfig({ kind: event.target.value as AiProviderKind })}>
              <option value="openai">OpenAI-compatible</option>
              <option value="anthropic">Anthropic Messages</option>
            </select></label>}
            <label className="field"><span>URL base</span><input className="input" value={config.baseUrl} onChange={(event) => updateConfig({ baseUrl: event.target.value })} placeholder="https://…/v1" disabled={busy}/></label>
            <label className="field"><span>Modelo</span><input className="input" list="ai-model-options" value={config.model} onChange={(event) => updateConfig({ model: event.target.value })} placeholder="identificador do modelo" disabled={busy}/>
              <datalist id="ai-model-options">{[...(selected?.models ?? []), ...models].filter((value, index, values) => value && values.indexOf(value) === index).map((model) => <option key={model} value={model}/>)}</datalist>
            </label>
            <label className="field"><span>{selected?.keyRequired === false ? 'Chave de API (opcional)' : 'Chave de API'}</span><input className="input" type="password" autoComplete="off" spellCheck={false} value={apiKey} onChange={(event) => setApiKey(event.target.value)} placeholder={hasKey ? 'Chave já salva; cole outra para substituir' : 'cole a chave aqui'} disabled={busy || savingKey}/></label>
            <div className="ai-form-actions">
              <Button primary disabled={!apiKey.trim() || savingKey || busy} onClick={() => void saveKey()}>{savingKey ? 'Salvando…' : 'Salvar chave'}</Button>
              {hasKey && <Button disabled={savingKey || busy} onClick={() => void deleteKey()}>Remover</Button>}
            </div>
            <div className="ai-form-actions">
              <Button disabled={testing || busy || !keyReady} onClick={() => void testProvider()}>{testing ? 'Testando…' : 'Testar conexão'}</Button>
              <Button disabled={discovering || busy} onClick={() => void discoverModels()}>{discovering ? 'Buscando…' : 'Descobrir modelos'}</Button>
            </div>
            {testResult && <div className="ai-test-result"><Badge tone="good">Conexão OK</Badge><span>{testResult.latencyMs} ms · {testResult.model}</span>{testResult.sample && <code>{testResult.sample}</code>}</div>}
            {selected && <p className="form-note">{selected.note}</p>}
          </div>
        </Panel>

        <Panel title="Contexto do caso" subtitle="Opcional; resumo estruturado e sem identificação direta." actions={selectedCase && attachCase ? <Badge tone="review">Anexado</Badge> : <Badge>Não anexado</Badge>}>
          {cases.length === 0 ? <p className="form-note">Nenhum cálculo não arquivado disponível. O Copiloto funciona sem contexto de caso.</p> : <>
            <label className="field"><span>Caso</span><select className="input" value={caseId ?? ''} onChange={(event) => { setCaseId(event.target.value ? Number(event.target.value) : null); setAttachCase(false); }} disabled={busy}>
              <option value="">Nenhum caso</option>
              {cases.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
            </select></label>
            {selectedCase && <>
              <label className="checkline"><input type="checkbox" checked={attachCase} onChange={(event) => setAttachCase(event.target.checked)} disabled={busy}/>Anexar este resumo à próxima pergunta</label>
              <details className="ai-case-preview"><summary>Ver o que seria enviado</summary><pre>{caseSummary(selectedCase)}</pre></details>
              <p className="form-note">O seletor mostra o nome apenas nesta tela. O resumo enviado omite nome, CPF, NIT, salários, notas e textos livres. Ele só sai da máquina se a caixa acima estiver marcada e o envio ao provedor estiver autorizado.</p>
            </>}
          </>}
        </Panel>

        <Panel title="Base local" subtitle="Ferramentas somente leitura; nenhum resultado é gravado." actions={<Badge tone={dataReady ? 'good' : 'review'}>{dataReady ? 'Importada' : 'Vazia'}</Badge>}>
          <div className="ai-data-grid">
            <div><strong>{dataStatus?.skills ?? 0}</strong><span>habilidades</span></div>
            <div><strong>{dataStatus?.legislation ?? 0}</strong><span>normas</span></div>
            <div><strong>{dataStatus?.natjus ?? 0}</strong><span>NatJus · {dataStatus?.natjusText ?? 0} com texto</span></div>
            <div><strong>{dataStatus?.jurisprudence ?? 0}</strong><span>ementas</span></div>
          </div>
          <p className="form-note">{dataStatus?.message ?? 'Lendo a base local…'}</p>
          <div className="ai-form-actions">
            <Button primary disabled={importing || busy} onClick={() => void importData()}>{importing ? 'Importando…' : 'Selecionar pasta da auditoria'}</Button>
            <Button disabled={importing} onClick={() => void api.aiDataStatus().then(setDataStatus).catch((error) => setNotice({ message: errorMessage(error), error: true }))}>Atualizar contagens</Button>
          </div>
          <label className="checkline"><input type="checkbox" checked={includePdf} onChange={(event) => setIncludePdf(event.target.checked)} disabled={importing || busy}/>Copiar PDFs NatJus e extrair texto (recomendado, cerca de 113 MB)</label>
          {!dataReady && <div className="ai-command"><span>Alternativa por terminal:</span><code>{command}</code></div>}
          <label className="checkline"><input type="checkbox" checked={toolsEnabled} onChange={(event) => setToolsEnabled(event.target.checked)} disabled={busy}/>Permitir consultas à base local nesta conversa</label>
          <p className="form-note">As consultas são feitas no seu computador. Os trechos encontrados voltam ao provedor como resultado da ferramenta para que o modelo possa citá-los.</p>
        </Panel>

        <Panel title="Limites e privacidade" subtitle="O Copiloto não recebe o workspace inteiro.">
          <ul className="ai-privacy-list">
            <li><Icon name="shield" size={15}/><span>O CNIS, os clientes, os cálculos e os documentos não são anexados automaticamente.</span></li>
            <li><Icon name="shield" size={15}/><span>Só o texto desta conversa e os trechos de ferramentas que você habilitar saem da máquina.</span></li>
            <li><Icon name="shield" size={15}/><span>A chave é protegida pelo DPAPI do Windows; o aplicativo não a exibe de volta.</span></li>
            <li><Icon name="shield" size={15}/><span>O histórico do chat não é salvo no workspace; limpar a página encerra a conversa.</span></li>
          </ul>
          <p className="form-note">Provedores externos podem reter e processar o texto conforme os próprios termos. Confirme a autorização de uso jurídico e a política de dados do provedor escolhido. CommandCode e Cline são agentes BYOK, não gateways universais; ferramentas de design não são provedores de modelo.</p>
        </Panel>
      </aside>
    </div>
  </div>;
}

