import { useEffect, useMemo, useRef, useState } from 'react';
import { api, errorMessage } from '../api';
import type { AiChatMessage, AiEvent, AiKeyStatus, AiProviderKind, AiProviderPreset, AiSource, AiTask } from '../api';
import { caseSummary, detectPromptInjection } from '../aiCase';
import type { Workspace } from '../model';
import { Badge, Button, Icon, Notice, Panel } from './ui';

interface StoredConfig { providerId: string; kind: AiProviderKind; baseUrl: string; model: string }
interface ToolActivity { name: string; status: 'running' | 'done' | 'error'; summary: string; sources: AiSource[] }
interface MinutaVersion { id: string; label: string; text: string; date: string }
export interface CaseDocument { fileName: string; text: string; usedOcr?: boolean; pages?: number; hashSha256?: string; reviewed?: boolean }
interface PecaClassification { area: string; materia: string; tecnicas: string }
interface MinutaDraftCache {
  objective: string;
  documentType: string;
  caseId: number | null;
  attachCase: boolean;
  caseDocument: CaseDocument | null;
  plan: string;
  draft: string;
  review: string;
  versions: MinutaVersion[];
  versionLabel: string;
  templateName: string;
  sources: AiSource[];
  classification: PecaClassification;
  classificationConfirmed: boolean;
}
const CONFIG_KEY = 'previdenciar.ai.config.v1';
const DOCUMENT_TYPES = ['Petição inicial', 'Manifestação', 'Parecer', 'Recurso', 'Contestação', 'Memorial', 'Outro'];
let draftCache: MinutaDraftCache | null = null;
export function resetMinutaDraftCache() { draftCache = null; }

function readConfig(): StoredConfig | null {
  try {
    const value = JSON.parse(localStorage.getItem(CONFIG_KEY) ?? 'null') as StoredConfig | null;
    if (!value || typeof value.providerId !== 'string') return null;
    return value;
  } catch { return null; }
}
function id(prefix: string): string {
  return `${prefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 9)}`;
}

export function MinutaPage({ workspace, notify, onOpenCopilot, onSaveTemplate, onSaveReading }: {
  workspace: Workspace;
  notify: (message: string, error?: boolean) => void;
  onOpenCopilot: () => void;
  onSaveTemplate: (name: string, body: string) => Promise<void>;
  onSaveReading?: (document: CaseDocument, caseId: number | null) => Promise<void>;
}) {
  const [catalog, setCatalog] = useState<AiProviderPreset[]>([]);
  const [keys, setKeys] = useState<Record<string, boolean>>({});
  const [config, setConfig] = useState<StoredConfig | null>(readConfig());
  const [consent, setConsent] = useState(false);
  const [caseId, setCaseId] = useState<number | null>(() => draftCache?.caseId ?? null);
  const [attachCase, setAttachCase] = useState(() => draftCache?.attachCase ?? true);
  const [caseDocument, setCaseDocument] = useState<CaseDocument | null>(() => draftCache?.caseDocument ?? null);
  const [extracting, setExtracting] = useState(false);
  const [savingReading, setSavingReading] = useState(false);
  const [documentType, setDocumentType] = useState(() => draftCache?.documentType ?? DOCUMENT_TYPES[0]);
  const [objective, setObjective] = useState(() => draftCache?.objective ?? '');
  const [plan, setPlan] = useState(() => draftCache?.plan ?? '');
  const [draft, setDraft] = useState(() => draftCache?.draft ?? '');
  const [review, setReview] = useState(() => draftCache?.review ?? '');
  const [versions, setVersions] = useState<MinutaVersion[]>(() => draftCache?.versions ?? []);
  const [versionLabel, setVersionLabel] = useState(() => draftCache?.versionLabel ?? '');
  const [templateName, setTemplateName] = useState(() => draftCache?.templateName ?? '');
  const [busy, setBusy] = useState<AiTask | null>(null);
  const [tools, setTools] = useState<ToolActivity[]>([]);
  const [sources, setSources] = useState<AiSource[]>(() => draftCache?.sources ?? []);
  const [notice, setNotice] = useState<{ message: string; error: boolean } | null>(null);
  const [classification, setClassification] = useState<PecaClassification>(() => draftCache?.classification ?? { area: '', materia: '', tecnicas: '' });
  const [classificationConfirmed, setClassificationConfirmed] = useState(() => draftCache?.classificationConfirmed ?? false);
  const activeRequest = useRef<string | null>(null);
  const classifyBuffer = useRef('');

  const cases = useMemo(() => workspace.calculations.filter((item) => !item.archived), [workspace.calculations]);
  const selectedCase = useMemo(() => cases.find((item) => item.id === caseId) ?? null, [cases, caseId]);
  const caseContext = useMemo(() => {
    const parts: string[] = [];
    if (selectedCase && attachCase) parts.push(caseSummary(selectedCase));
    if (caseDocument?.reviewed) parts.push(`TEXTO EXTRAÍDO DO DOCUMENTO (${caseDocument.fileName}):\n${caseDocument.text}`);
    return parts.length ? parts.join('\n\n') : null;
  }, [selectedCase, attachCase, caseDocument]);
  const attachmentWarning = useMemo(() => caseDocument ? detectPromptInjection(caseDocument.text) : null, [caseDocument]);
  const savedReadings = useMemo(() => {
    if (!selectedCase) return [];
    return selectedCase.reading_ids.map((id) => workspace.readings.find((reading) => reading.id === id)).filter((reading) => reading !== undefined);
  }, [selectedCase, workspace.readings]);
  const provider = useMemo(() => catalog.find((item) => item.id === config?.providerId) ?? null, [catalog, config]);
  const classificationLine = useMemo(() => classificationConfirmed
    ? `\nClassificação confirmada da peça — Área: ${classification.area}; Matéria: ${classification.materia}; Técnicas: ${classification.tecnicas || 'não informadas'}.`
    : '', [classificationConfirmed, classification]);
  const keyReady = config ? (keys[config.providerId] || provider?.keyRequired === false) : false;
  const attachmentReviewed = !caseDocument || Boolean(caseDocument.reviewed);
  const ready = Boolean(config && keyReady && consent && config.model.trim() && config.baseUrl.trim() && attachmentReviewed);
  const citations = useMemo(() => {
    const refs = [...new Set(Array.from(draft.matchAll(/\[([HLNJ]\d+)\]/g), (match) => match[1]))];
    const available = new Set(sources.map((source) => source.label));
    return refs.map((ref) => ({ ref, verified: available.has(ref) }));
  }, [draft, sources]);
  const readiness = useMemo(() => {
    const pending = Array.from(draft.matchAll(/\[(?:PENDENTE|A CONFERIR|VERIFICAR)[^\]]*\]/gi)).length;
    const emptySections: string[] = [];
    let current: string | null = null;
    let hasContent = false;
    let firstHeading = true;
    for (const line of draft.split(/\r?\n/)) {
      const heading = line.match(/^(#{1,3})\s+(.+)/);
      if (heading) {
        if (current && !hasContent) emptySections.push(current);
        const isDocumentTitle = firstHeading && heading[1].length === 1;
        firstHeading = false;
        current = isDocumentTitle ? null : heading[2].trim();
        hasContent = false;
        continue;
      }
      if (current && line.trim()) hasContent = true;
    }
    if (current && !hasContent) emptySections.push(current);
    const unverified = citations.filter((item) => !item.verified).length;
    const issues: string[] = [];
    if (pending) issues.push(`${pending} marcação(ões) de pendência`);
    if (emptySections.length) issues.push(`${emptySections.length} seção(ões) sem texto: ${emptySections.slice(0, 3).join('; ')}`);
    if (unverified) issues.push(`${unverified} citação(ões) não retornada(s) nesta sessão`);
    if (caseDocument && !caseDocument.reviewed) issues.push('leitura do anexo ainda não confirmada');
    if (!review.trim()) issues.push('revisão crítica ainda não executada');
    return { issues, pending, emptySections, unverified, reviewed: Boolean(review.trim()) };
  }, [draft, citations, review, caseDocument]);

  useEffect(() => {
    let alive = true;
    void Promise.all([api.aiCatalog(), api.aiKeyStatus()]).then(([providers, statuses]) => {
      if (!alive) return;
      setCatalog(providers);
      setKeys(Object.fromEntries(statuses.map((item: AiKeyStatus) => [item.providerId, item.hasKey])));
      setConfig((current) => {
        if (current) {
          const found = providers.find((item) => item.id === current.providerId);
          return found ? current : providers[0] ? { providerId: providers[0].id, kind: providers[0].kind, baseUrl: providers[0].baseUrl, model: providers[0].defaultModel } : null;
        }
        const fallback = providers.find((item) => item.id === 'opencode-go') ?? providers[0];
        return fallback ? { providerId: fallback.id, kind: fallback.kind, baseUrl: fallback.baseUrl, model: fallback.defaultModel } : null;
      });
    }).catch((error) => setNotice({ message: errorMessage(error), error: true }));
    return () => { alive = false; };
  }, []);

  useEffect(() => {
    draftCache = { objective, documentType, caseId, attachCase, caseDocument, plan, draft, review, versions, versionLabel, templateName, sources, classification, classificationConfirmed };
  }, [objective, documentType, caseId, attachCase, caseDocument, plan, draft, review, versions, versionLabel, templateName, sources, classification, classificationConfirmed]);

  useEffect(() => () => {
    const requestId = activeRequest.current;
    if (requestId) void api.aiCancel(requestId).catch(() => {});
  }, []);

  type TaskSink = 'plan' | 'draft' | 'review' | 'classification' | 'ignore';
  async function runTask(task: AiTask, messages: AiChatMessage[], maxTokens: number, sinkOverride?: TaskSink) {
    const sink: TaskSink = sinkOverride ?? (task === 'plan' || task === 'draft' ? task : task === 'review' || task === 'adversarial' ? 'review' : 'ignore');
    if (!config) { setNotice({ message: 'Configure um provedor no Copiloto IA antes de gerar a minuta.', error: true }); return; }
    if (!consent) { setNotice({ message: 'Marque a autorização de envio ao provedor.', error: true }); return; }
    if (!keyReady) { setNotice({ message: 'Salve a chave do provedor no Copiloto IA ou escolha um endpoint local sem chave.', error: true }); return; }
    if (!attachmentReviewed) { setNotice({ message: 'Confirme a leitura do anexo antes de enviar a minuta ao provedor.', error: true }); return; }
    setBusy(task);
    setTools([]);
    setNotice(null);
    if (sink === 'plan') setPlan('');
    if (sink === 'draft') {
      pushVersion(draft, 'Antes de gerar nova minuta');
      setDraft('');
    }
    if (sink === 'review') setReview('');
    if (sink === 'classification') classifyBuffer.current = '';
    const requestId = id(`minuta-${task}`);
    activeRequest.current = requestId;
    try {
      await api.aiChat({ requestId, providerId: config.providerId, kind: config.kind, baseUrl: config.baseUrl, model: config.model, messages, consent: true, toolsEnabled: true, maxTokens, caseContext, task }, (event: AiEvent) => {
        if (event.type === 'delta') {
          if (sink === 'plan') setPlan((current) => current + event.text);
          if (sink === 'draft') setDraft((current) => current + event.text);
          if (sink === 'review') setReview((current) => current + event.text);
          if (sink === 'classification') classifyBuffer.current += event.text;
        }
        if (event.type === 'tool') {
          setTools((current) => {
            const running = current.findIndex((item) => item.name === event.name && item.status === 'running');
            const activity: ToolActivity = { name: event.name, status: event.status, summary: event.summary, sources: event.sources };
            if (running >= 0) return current.map((item, index) => index === running ? activity : item);
            return [...current, activity];
          });
          setSources((current) => {
            const next = [...current];
            for (const source of event.sources) if (!next.some((item) => item.label === source.label && item.title === source.title)) next.push(source);
            return next;
          });
        }
      });
      if (sink === 'classification') {
        const parsed = parseClassification(classifyBuffer.current);
        if (!parsed.area && !parsed.materia && !parsed.tecnicas) setNotice({ message: 'A proposta da IA não pôde ser interpretada. Preencha a classificação manualmente.', error: true });
        else {
          setClassification(parsed);
          setClassificationConfirmed(false);
          setNotice({ message: 'Classificação proposta pela IA. Revise os campos e confirme antes de redigir.', error: false });
        }
      }
    } catch (error) {
      const message = errorMessage(error);
      if (!/cancelad/i.test(message)) setNotice({ message, error: true });
      else notify('Geração cancelada.');
    } finally { activeRequest.current = null; setBusy(null); }
  }
  function parseClassification(text: string): PecaClassification {
    const field = (label: string) => {
      const match = text.match(new RegExp(`^\\s*(?:[-*•]\\s*)?${label}\\s*:\\s*(.+)$`, 'im'));
      return match ? match[1].trim().replace(/[.;]+$/, '') : '';
    };
    return { area: field('Área'), materia: field('Matéria'), tecnicas: field('Técnicas') };
  }
  function updateClassification(patch: Partial<PecaClassification>) {
    setClassification((current) => ({ ...current, ...patch }));
    setClassificationConfirmed(false);
  }
  function confirmClassification() {
    if (!classification.area.trim() || !classification.materia.trim()) {
      setNotice({ message: 'Informe a área e a matéria da peça antes de confirmar a classificação.', error: true });
      return;
    }
    setClassificationConfirmed(true);
    setNotice({ message: 'Classificação confirmada por você. A redação da minuta está liberada.', error: false });
  }
  function proposeClassification() {
    if (!objective.trim()) { setNotice({ message: 'Descreva o objetivo da minuta antes de pedir a classificação.', error: true }); return; }
    void runTask('chat', [{ role: 'user', content: `Tipo de documento: ${documentType}\nObjetivo: ${objective.trim()}\n\nClassifique a peça jurídica que será redigida. Responda somente neste formato, uma informação por linha, sem comentários:\nÁrea: <área do direito>\nMatéria: <matéria principal da peça>\nTécnicas: <técnicas processuais, separadas por ponto e vírgula>` }], 300, 'classification');
  }
  async function attachPdf() {
    setExtracting(true);
    setNotice({ message: 'Extraindo texto localmente. Se o PDF for escaneado, o OCR pode demorar.', error: false });
    try {
      const document = await api.pickCasePdfText();
      if (!document) { setNotice(null); return; }
      setCaseDocument({ ...document, reviewed: false });
      const origin = document.usedOcr ? `OCR local (${document.pages} página(s))` : 'texto do PDF';
      setNotice({ message: `${origin} de ${document.fileName}: ${document.text.length} caracteres extraídos; documento ${document.hashSha256.slice(0, 8)}.`, error: false });
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
    finally { setExtracting(false); }
  }
  async function saveReading() {
    if (!caseDocument || !caseDocument.reviewed) { setNotice({ message: "Confirme a leitura do anexo antes de salvar.", error: true }); return; }
    if (!caseId) { setNotice({ message: "Vincule um caso antes de salvar a leitura.", error: true }); return; }
    if (!onSaveReading) return;
    setSavingReading(true);
    try {
      await onSaveReading(caseDocument, caseId);
      notify("Leitura salva no caso com hash e origem.");
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
    finally { setSavingReading(false); }
  }
  async function cancel() {
    if (!activeRequest.current) return;
    try { await api.aiCancel(activeRequest.current); notify('Cancelamento solicitado.'); }
    catch (error) { setNotice({ message: errorMessage(error), error: true }); }
  }

  function makePlan() {
    if (!objective.trim()) { setNotice({ message: 'Descreva o objetivo da minuta antes de gerar o roteiro.', error: true }); return; }
    void runTask('plan', [{ role: 'user', content: `Tipo de documento: ${documentType}\nObjetivo: ${objective.trim()}${classificationLine}\n\nProduza o roteiro de planejamento. Se faltarem dados essenciais, liste perguntas objetivas antes da tese.` }], 1800);
  }
  function makeDraft() {
    if (!plan.trim()) { setNotice({ message: 'Gere ou escreva um roteiro antes de redigir a minuta.', error: true }); return; }
    if (!classificationConfirmed) { setNotice({ message: 'Confirme a classificação da peça (área, matéria e técnicas) antes de redigir.', error: true }); return; }
    void runTask('draft', [{ role: 'user', content: `Tipo de documento: ${documentType}\nObjetivo: ${objective.trim()}${classificationLine}\n\nRoteiro aprovado:\n${plan.trim()}\n\nRedija a minuta em Markdown, seguindo o roteiro e marcando pendências.` }], 4000);
  }
  function makeAdversarial() {
    if (!draft.trim()) { setNotice({ message: 'Gere ou escreva uma minuta antes da revisão adversarial.', error: true }); return; }
    void runTask('adversarial', [{ role: 'user', content: `Tipo de documento: ${documentType}\nObjetivo: ${objective.trim()}\n\nMinuta para revisão adversarial:\n${draft.trim()}\n\nProduza o relatório priorizado e os três riscos mais graves.` }], 2200);
  }
  function makeReview() {
    if (!draft.trim()) { setNotice({ message: 'Gere ou escreva uma minuta antes de revisar.', error: true }); return; }
    void runTask('review', [{ role: 'user', content: `Tipo de documento: ${documentType}\nObjetivo: ${objective.trim()}\n\nMinuta para revisão:\n${draft.trim()}\n\nProduza o checklist de revisão e os três pontos mais urgentes.` }], 2000);
  }
  async function exportDraft() {
    if (!draft.trim()) return;
    try {
      const path = await api.exportDocument(draft);
      if (path) notify('Minuta exportada em texto. Revise antes de usar.');
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
  }
  async function exportDocx() {
    if (!draft.trim()) return;
    const { Document, HeadingLevel, Packer, Paragraph } = await import('docx');
    const clean = (value: string) => value.replace(/\*\*(.*?)\*\*/g, '$1').replace(/`/g, '').trim();
    const children = draft.split(/\r?\n/).map((line) => {
      const text = clean(line);
      if (!text) return new Paragraph({ text: '' });
      if (/^###\s+/.test(text)) return new Paragraph({ text: text.replace(/^###\s+/, ''), heading: HeadingLevel.HEADING_3 });
      if (/^##\s+/.test(text)) return new Paragraph({ text: text.replace(/^##\s+/, ''), heading: HeadingLevel.HEADING_2 });
      if (/^#\s+/.test(text)) return new Paragraph({ text: text.replace(/^#\s+/, ''), heading: HeadingLevel.HEADING_1 });
      if (/^[-*]\s+/.test(text)) return new Paragraph({ text: text.replace(/^[-*]\s+/, ''), bullet: { level: 0 } });
      return new Paragraph({ text });
    });
    try {
      const blob = await Packer.toBlob(new Document({ sections: [{ children }] }));
      const bytes = new Uint8Array(await blob.arrayBuffer());
      const safe = (templateName.trim() || documentType).normalize('NFD').replace(/[\u0300-\u036f]/g, '').replace(/[^A-Za-z0-9 _()-]/g, '').slice(0, 80) || 'minuta';
      const path = await api.saveExportFile(bytes, 'docx', safe);
      if (path) notify('Minuta DOCX exportada. Revise antes de usar.');
    } catch (error) { setNotice({ message: errorMessage(error), error: true }); }
  }
  function printDraft() {
    if (!draft.trim()) return;
    const escape = (value: string) => value.replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' } as Record<string, string>)[character] ?? character);
    const inline = (value: string) => escape(value).replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>').replace(/`([^`]+)`/g, '<code>$1</code>');
    const body = draft.split(/\r?\n/).map((line) => {
      if (!line.trim()) return '';
      if (/^###\s+/.test(line)) return `<h3>${inline(line.replace(/^###\s+/, ''))}</h3>`;
      if (/^##\s+/.test(line)) return `<h2>${inline(line.replace(/^##\s+/, ''))}</h2>`;
      if (/^#\s+/.test(line)) return `<h1>${inline(line.replace(/^#\s+/, ''))}</h1>`;
      if (/^[-*]\s+/.test(line)) return `<p class="bullet">• ${inline(line.replace(/^[-*]\s+/, ''))}</p>`;
      return `<p>${inline(line)}</p>`;
    }).join('');
    const html = `<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>${escape(templateName || documentType)}</title><style>body{font-family:Georgia,serif;font-size:12pt;line-height:1.65;margin:3cm 2.5cm;color:#111}h1,h2,h3{line-height:1.3;margin:1.4em 0 .6em}h1{font-size:16pt}h2{font-size:14pt}h3{font-size:12.5pt}p{margin:0 0 .8em;text-align:justify}.bullet{margin-left:1.2em;text-align:left}code{font-family:Consolas,monospace;font-size:10.5pt}</style></head><body>${body}</body></html>`;
    const url = URL.createObjectURL(new Blob([html], { type: 'text/html' }));
    const frame = document.createElement('iframe');
    frame.style.cssText = 'position:fixed;right:0;bottom:0;width:0;height:0;border:0;';
    frame.src = url;
    document.body.appendChild(frame);
    frame.onload = () => {
      frame.contentWindow?.focus();
      frame.contentWindow?.print();
      setTimeout(() => { URL.revokeObjectURL(url); frame.remove(); }, 60000);
    };
  }
  async function saveTemplate() {
    if (!draft.trim()) { setNotice({ message: 'Gere ou escreva uma minuta antes de salvar o modelo.', error: true }); return; }
    const name = templateName.trim() || `${documentType}${selectedCase ? ` - ${selectedCase.name}` : ''}`;
    try {
      await onSaveTemplate(name, draft);
      setTemplateName('');
      notify('Minuta salva como modelo local no workspace.');
    } catch { /* the app already reported the failure */ }
  }
  function pushVersion(text: string, label: string) {
    if (!text.trim()) return;
    setVersions((current) => {
      if (current[0]?.text === text) return current;
      return [{ id: id('version'), label, text, date: new Date().toLocaleString('pt-BR') }, ...current].slice(0, 20);
    });
  }
  function saveVersion() {
    if (!draft.trim()) { setNotice({ message: 'Gere ou escreva uma minuta antes de salvar uma versão.', error: true }); return; }
    pushVersion(draft, versionLabel.trim() || `Versão ${versions.length + 1}`);
    setVersionLabel('');
    notify('Versão salva nesta sessão.');
  }
  function restoreVersion(version: { text: string; label: string }) {
    pushVersion(draft, 'Antes de restaurar versão');
    setDraft(version.text);
    notify(`Versão “${version.label}” restaurada no editor.`);
  }
  function copy(text: string, label: string) {
    void navigator.clipboard?.writeText(text).then(() => notify(label)).catch(() => notify('Não foi possível copiar.', true));
  }

  return <div className="minuta-page">
    <div className="minuta-banner">
      <div className="minuta-banner-mark"><Icon name="file" size={22}/></div>
      <div><strong>Minuta assistida em blocos</strong><p>Planeje, redija e revise com o provedor que você configurou. O texto é um rascunho; a decisão e a conferência continuam com você. Dados ausentes viram pendências, não invenção.</p></div>
      <Badge tone={keyReady ? 'good' : 'review'}>{keyReady ? 'Provedor pronto' : 'Configurar provedor'}</Badge>
    </div>

    {!keyReady && <Notice tone="error">Não há chave disponível para este provedor. Abra o Copiloto IA, salve a chave e volte. <Button onClick={onOpenCopilot}>Abrir Copiloto IA</Button></Notice>}

    <div className="minuta-top">
      <Panel title="Caso e objetivo" subtitle="O resumo do caso é opcional e não inclui identificação direta.">
        <div className="minuta-form">
          <label className="field"><span>Caso</span><select className="input" value={caseId ?? ''} onChange={(event) => setCaseId(event.target.value ? Number(event.target.value) : null)} disabled={busy !== null}>
            <option value="">Sem caso vinculado</option>
            {cases.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
          </select></label>
          {selectedCase && <label className="checkline wide"><input type="checkbox" checked={attachCase} onChange={(event) => setAttachCase(event.target.checked)}/>Incluir o resumo estrutural deste caso no envio</label>}
          <div className="minuta-pdf-attach wide">
            <Button onClick={() => void attachPdf()} disabled={busy !== null || extracting}>{extracting ? 'Extraindo texto…' : 'Anexar PDF do caso (texto/OCR)'}</Button>
            {caseDocument ? <span>{caseDocument.fileName} · {caseDocument.usedOcr ? `OCR local em ${caseDocument.pages ?? 0} página(s)` : 'texto do PDF'} · {caseDocument.text.length} caracteres · hash {caseDocument.hashSha256?.slice(0, 8) ?? '—'} · {caseDocument.reviewed ? 'leitura confirmada' : 'leitura em revisão'}. O texto pode conter dados pessoais; revise antes de enviar. <button type="button" className="link-button" onClick={() => setCaseDocument(null)}>remover</button></span> : <span>Extrai texto de PDF textual ou aplica OCR local em digitalizações quando Tesseract e Poppler estão instalados. O texto pode conter dados pessoais; revise antes de enviar.</span>}
          </div>
          {caseId && savedReadings.length > 0 && <details className="minuta-case-preview wide"><summary>Leituras salvas neste caso ({savedReadings.length})</summary><ul className="minuta-saved-readings">{savedReadings.map((reading) => <li key={reading.id}><strong>{workspace.source_documents.find((document) => document.id === reading.documento_id)?.nome_arquivo ?? reading.documento_id}</strong><span>{reading.status === "confirmada" ? "confirmada" : "em revisão"} · {reading.paginas_lidas} página(s)</span></li>)}</ul></details>}
          {attachmentWarning && <div className="minuta-notice is-error wide" role="alert"><Icon name="shield" size={17}/><span><strong>Possível instrução dirigida ao modelo.</strong> O trecho “{attachmentWarning}” foi encontrado no texto extraído. Ele continua sendo tratado como dado, mas revise e remova o anexo se não for necessário.</span></div>}
          {caseDocument && <label className="checkline wide"><input type="checkbox" checked={Boolean(caseDocument.reviewed)} onChange={(event) => setCaseDocument((current) => current ? { ...current, reviewed: event.target.checked } : current)} disabled={busy !== null || extracting}/>Revisei o texto extraído e confirmo a leitura deste anexo</label>}
          {caseDocument && !caseDocument.reviewed && <p className="form-note wide">A geração fica bloqueada até a confirmação da leitura do anexo.</p>}
          {onSaveReading && caseDocument && <div className="minuta-actions wide"><Button disabled={!caseId || !caseDocument.reviewed || savingReading} onClick={() => void saveReading()}>{savingReading ? "Salvando leitura…" : "Salvar leitura no caso"}</Button></div>}
          {caseDocument && <details className="minuta-case-preview wide"><summary>Ver texto extraído do anexo ({caseDocument.text.length} caracteres)</summary><pre>{caseDocument.text}</pre></details>}
          <label className="field"><span>Tipo de documento</span><select className="input" value={documentType} onChange={(event) => setDocumentType(event.target.value)} disabled={busy !== null}>{DOCUMENT_TYPES.map((type) => <option key={type}>{type}</option>)}</select></label>
          <label className="field wide"><span>Objetivo e limites</span><textarea className="text-area" value={objective} onChange={(event) => setObjective(event.target.value)} placeholder="Ex.: manifestar sobre a diligência registral, sem pedir suspensão, delimitando os eventos e as fontes." disabled={busy !== null}/></label>
          <label className="checkline wide"><input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} disabled={busy !== null}/>Permitir o envio deste caso e desta minuta ao provedor</label>
          {caseContext && <details className="minuta-case-preview wide"><summary>Ver resumo do caso que será enviado</summary><pre>{caseContext}</pre></details>}
        </div>
      </Panel>
      <Panel title="Ações" subtitle="Cada etapa usa o mesmo provedor; o resultado fica editável.">
        <div className="minuta-actions">
          <Button primary disabled={!ready || busy !== null} onClick={makePlan}><Icon name="spark" size={16}/>{busy === 'plan' ? 'Planejando…' : '1. Gerar roteiro'}</Button>
          {busy !== null && <Button onClick={() => void cancel()}>Cancelar geração</Button>}
          <Button disabled={!ready || !plan.trim() || !classificationConfirmed || busy !== null} title={!classificationConfirmed ? 'Confirme a classificação da peça para liberar a redação.' : undefined} onClick={makeDraft}><Icon name="file" size={16}/>{busy === 'draft' ? 'Redigindo…' : '2. Gerar minuta'}</Button>
          <Button disabled={!ready || !draft.trim() || busy !== null} onClick={makeReview}><Icon name="shield" size={16}/>{busy === 'review' ? 'Revisando…' : '3. Revisar'}</Button>
          <Button disabled={!ready || !draft.trim() || busy !== null} onClick={makeAdversarial}><Icon name="shield" size={16}/>{busy === 'adversarial' ? 'Atacando…' : '4. Revisão adversarial'}</Button>
        </div>
        {!classificationConfirmed && <p className="form-note">A redação fica bloqueada até a confirmação da classificação da peça, na etapa “Classificação da peça”.</p>}
        <div className="minuta-actions">
          <Button disabled={!draft.trim() || busy !== null} onClick={() => copy(draft, 'Minuta copiada.')}>Copiar minuta</Button>
          <Button disabled={!draft.trim() || busy !== null} onClick={() => void exportDraft()}>Exportar texto</Button>
          <Button disabled={!draft.trim() || busy !== null} onClick={printDraft}>Imprimir / PDF</Button>
          <Button disabled={!draft.trim() || busy !== null} onClick={() => void exportDocx()}>Exportar DOCX</Button>
        </div>
        <div className="minuta-save">
          <label className="field"><span>Salvar como modelo local</span><input className="input" value={templateName} onChange={(event) => setTemplateName(event.target.value)} placeholder="Nome do modelo (opcional)"/></label>
          <Button disabled={!draft.trim() || busy !== null} onClick={saveTemplate}>Salvar no workspace</Button>
        </div>
        <p className="form-note">O workspace guarda o texto como modelo local. A exportação em texto, DOCX e PDF (via diálogo de impressão) é sempre um rascunho para revisão profissional.</p>
      </Panel>
    </div>

    <Panel title="Classificação da peça" actions={<Badge tone={classificationConfirmed ? 'good' : (classification.area.trim() || classification.materia.trim()) ? 'review' : 'neutral'}>{classificationConfirmed ? 'Classificação confirmada' : 'A confirmar'}</Badge>}>
      <div className="minuta-classification">
        <label className="field"><span>Área</span><input className="input" value={classification.area} onChange={(event) => updateClassification({ area: event.target.value })} placeholder="Ex.: Direito Previdenciário" disabled={busy !== null}/></label>
        <label className="field"><span>Matéria</span><input className="input" value={classification.materia} onChange={(event) => updateClassification({ materia: event.target.value })} placeholder="Ex.: Aposentadoria por idade, revisão de DIB" disabled={busy !== null}/></label>
        <label className="field wide"><span>Técnicas</span><input className="input" value={classification.tecnicas} onChange={(event) => updateClassification({ tecnicas: event.target.value })} placeholder="Ex.: petição inicial; tutela de urgência; perícia técnica" disabled={busy !== null}/></label>
        <div className="minuta-classification__actions wide" data-ui-actions>
          <Button disabled={!ready || busy !== null || !objective.trim()} onClick={proposeClassification}><Icon name="spark" size={16}/>{busy === 'chat' ? 'Classificando…' : 'Propor classificação com a IA'}</Button>
          <Button primary disabled={!classification.area.trim() || !classification.materia.trim() || busy !== null} onClick={confirmClassification}><Icon name="check" size={16}/>Confirmar classificação</Button>
        </div>
        {classificationConfirmed
          ? <p className="minuta-classification__confirmed" role="status"><Icon name="check" size={15}/>Classificação confirmada por você. Área: {classification.area} · Matéria: {classification.materia}{classification.tecnicas ? ` · Técnicas: ${classification.tecnicas}` : ''}. A redação está liberada.</p>
          : <p className="form-note">A redação fica bloqueada até você confirmar a classificação. Preencha os campos manualmente ou peça uma proposta à IA. Qualquer alteração depois da confirmação exige confirmar de novo.</p>}
      </div>
    </Panel>

    {tools.length > 0 && <div className="minuta-tools" aria-label="Ferramentas consultadas">
      {tools.map((tool, index) => <span key={`${tool.name}-${index}`} className={`minuta-tool ${tool.status}`}><Icon name="tool" size={13}/><strong>{tool.name.replaceAll('_', ' ')}</strong>{tool.summary}</span>)}
    </div>}

    {notice && <div className={`minuta-notice ${notice.error ? 'is-error' : ''}`} role={notice.error ? 'alert' : 'status'}><Icon name={notice.error ? 'shield' : 'check'} size={17}/><span>{notice.message}</span><button type="button" className="iconbtn" aria-label="Fechar aviso" onClick={() => setNotice(null)}><Icon name="close" size={14}/></button></div>}

    <div className="minuta-workspace">
      <Panel title="Roteiro" subtitle="Edite livremente antes de liberar a redação." actions={<Badge tone={plan.trim() ? 'good' : 'neutral'}>{plan.trim() ? 'Rascunho' : 'Vazio'}</Badge>}>
        <textarea className="minuta-editor" value={plan} onChange={(event) => setPlan(event.target.value)} placeholder="O roteiro gerado aparece aqui. Você pode editar, remover ou acrescentar etapas antes de redigir." disabled={busy === 'plan'}/>
      </Panel>
      <Panel title="Minuta" subtitle="A peça gerada é um rascunho para revisão profissional." actions={<Badge tone={draft.trim() ? 'review' : 'neutral'}>{draft.trim() ? 'Rascunho' : 'Vazio'}</Badge>}>
        <textarea className="minuta-editor minuta-editor-large" value={draft} onChange={(event) => setDraft(event.target.value)} placeholder="A minuta aparece aqui depois do roteiro. Você pode editar o texto e marcar pendências." disabled={busy === 'draft'}/>
      </Panel>
    </div>

    <Panel title="Prontidão para revisão" subtitle="Checagem local antes de exportar; não substitui a leitura humana." actions={<Badge tone={draft.trim() && readiness.issues.length === 0 ? 'good' : readiness.issues.length ? 'review' : 'neutral'}>{draft.trim() ? (readiness.issues.length ? `${readiness.issues.length} aviso(s)` : 'Sem avisos locais') : 'Sem minuta'}</Badge>}>
      {!draft.trim() ? <p className="form-note">Gere ou escreva uma minuta para verificar pendências, seções vazias e citações.</p> : readiness.issues.length ? <>
        <ul className="minuta-readiness">{readiness.issues.map((issue) => <li key={issue}>{issue}</li>)}</ul>
        <p className="form-note">Os avisos não bloqueiam a exportação, mas evitam que uma peça incompleta seja tratada como pronta. Confira a fonte oficial e a revisão profissional.</p>
      </> : <p className="form-note">Nenhuma pendência local detectada. A conferência humana, a vigência e a fonte oficial continuam obrigatórias.</p>}
    </Panel>
    <Panel title="Versões desta sessão" subtitle="Guardadas apenas na memória desta página; exporte ou salve no workspace para persistir." actions={<Badge tone={versions.length ? 'neutral' : 'neutral'}>{versions.length} versões</Badge>}>
      <div className="minuta-version-add">
        <input className="input" value={versionLabel} onChange={(event) => setVersionLabel(event.target.value)} placeholder="Nome da versão (opcional)"/>
        <Button disabled={!draft.trim()} onClick={saveVersion}>Salvar versão</Button>
      </div>
      {versions.length === 0 ? <p className="form-note">Nenhuma versão salva nesta sessão.</p> : <div className="minuta-version-list">
        {versions.map((version) => <div key={version.id} className="minuta-version"><div><strong>{version.label}</strong><small>{version.date} · {version.text.length} caracteres</small></div><div className="minuta-version-actions"><Button onClick={() => restoreVersion(version)}>Restaurar</Button><Button onClick={() => void navigator.clipboard?.writeText(version.text).then(() => notify('Versão copiada.')).catch(() => notify('Não foi possível copiar.', true))}>Copiar</Button></div></div>)}
      </div>}
    </Panel>
    <Panel title="Conferência de citações" subtitle="Verificação local dos rótulos usados na minuta." actions={<Badge tone={citations.length && citations.every((item) => item.verified) ? 'good' : citations.length ? 'review' : 'neutral'}>{citations.length ? `${citations.filter((item) => item.verified).length}/${citations.length}` : 'Sem citações'}</Badge>}>
      {citations.length === 0 ? <p className="form-note">A minuta ainda não contém rótulos como [H1], [L1], [N1] ou [J1].</p> : <div className="minuta-citations">
        {citations.map((item) => <span key={item.ref} className={item.verified ? 'verified' : 'unverified'}>{item.ref} · {item.verified ? 'fonte retornada nesta sessão' : 'não retornada nesta sessão'}</span>)}
      </div>}
      <p className="form-note">“Retornada nesta sessão” não significa fonte oficial conferida. Abra a Base de conhecimento e o link oficial antes de usar a citação.</p>
    </Panel>
    <Panel title="Revisão crítica" subtitle="Lista de riscos, citações e dados faltantes." actions={<Badge tone={review.trim() ? 'review' : 'neutral'}>{review.trim() ? 'Revisão' : 'Não executada'}</Badge>}>
      <pre className="minuta-review">{review || 'Execute a revisão depois de gerar a minuta. Ela não substitui a leitura humana.'}</pre>
    </Panel>
  </div>;
}
















