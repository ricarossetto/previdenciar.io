import { useEffect, useMemo, useRef, useState } from 'react';
import type { Calculation, Json, Workspace } from '../model';
import { isoDate, money, parseMoney } from '../model';
import { api, errorMessage, type FabricaStatus } from '../api';
import { Badge, Button, Field, Icon, Modal, Notice } from './ui';
import type { FabricaSummary } from '../fabrica';
import {
  buildFabricaInput,
  buildFabricaInputDirect,
  difference,
  FABRICA_DOC_TYPES,
  FABRICA_ESPECIES,
  FABRICA_INDEX_TYPES,
  localRmi,
} from '../fabrica';

type FabricaTab = 'rmi' | 'tempo' | 'atrasados' | 'documentos' | 'indices';

const WEB_MODE = import.meta.env.MODE === 'web';

const FABRICA_TABS: { id: FabricaTab; label: string }[] = [
  { id: 'rmi', label: 'RMI (17 Espécies)' },
  { id: 'tempo', label: 'Tempo de Contribuição' },
  { id: 'atrasados', label: 'Atrasados / Liquidação' },
  { id: 'documentos', label: 'Leitor Documental' },
  { id: 'indices', label: 'Índices e Tabelas' },
];

function asObj(v: unknown): Record<string, unknown> | null {
  return v !== null && v !== undefined && typeof v === 'object' && !Array.isArray(v)
    ? (v as Record<string, unknown>)
    : null;
}
function asArr(v: unknown): unknown[] {
  return Array.isArray(v) ? v : [];
}
function asStr(v: unknown): string {
  return v === null || v === undefined ? '' : String(v);
}
function isoOrEmpty(value?: string | null): string {
  if (!value) return '';
  const text = value.trim();
  if (/^\d{4}-\d{2}-\d{2}$/.test(text)) return text;
  return isoDate(text) ?? '';
}

function parseRmiInput(raw: string): number | null {
  const clean = raw.trim().replace(/^R\$\s*/i, '').replace(/\s+/g, '');
  if (!clean) return null;
  if (clean.includes(',')) {
    const normalized = clean.replace(/\./g, '').replace(',', '.');
    const num = Number(normalized);
    return Number.isFinite(num) && num > 0 ? Number(num.toFixed(2)) : null;
  }
  if (/^\d+(?:\.\d+)?$/.test(clean)) {
    const num = Number(clean);
    return Number.isFinite(num) && num > 0 ? Number(num.toFixed(2)) : null;
  }
  const viaModel = parseMoney(clean);
  if (viaModel) {
    const num = Number(viaModel);
    return Number.isFinite(num) && num > 0 ? Number(num.toFixed(2)) : null;
  }
  return null;
}

function when(value: string | undefined | null): string {
  if (!value) return '—';
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString('pt-BR');
}
function amount(value: string | undefined | null): string {
  return value ? 'R$ ' + money(String(value)) : '—';
}
function coefficient(value: unknown): string {
  const number = typeof value === 'number' ? value : Number(value);
  return Number.isFinite(number)
    ? (number * 100).toLocaleString('pt-BR', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + '%'
    : '—';
}
function shortHash(value: string | undefined | null): string {
  return value ? value.slice(0, 16) + '…' : '—';
}
function percent(value: string): string {
  const number = Number(value);
  return Number.isFinite(number)
    ? (number > 0 ? '+' : '') + number.toLocaleString('pt-BR', { minimumFractionDigits: 1, maximumFractionDigits: 1 }) + '%'
    : value;
}
function deltaMoney(value: bigint): string {
  const negative = value < 0n;
  const absolute = negative ? -value : value;
  return (negative ? '-R$ ' : 'R$ ') + money((Number(absolute) / 100).toFixed(2));
}

function formatDaysBreakdown(daysRaw: unknown): string {
  const days = typeof daysRaw === 'number' ? daysRaw : Number(daysRaw);
  if (!Number.isFinite(days) || days < 0) return '—';
  const years = Math.floor(days / 365);
  const remAfterYears = days % 365;
  const months = Math.floor(remAfterYears / 30);
  const remDays = remAfterYears % 30;
  return `${years}a ${months}m ${remDays}d (${days} dias)`;
}

async function downloadBase64Pdf(base64Raw: string, suggestedName: string): Promise<void> {
  const clean = base64Raw.replace(/^data:application\/pdf;base64,/i, '').trim();
  const decoded = atob(clean);
  const bytes = Uint8Array.from(decoded, c => c.charCodeAt(0));
  if (WEB_MODE) {
    const blob = new Blob([bytes], { type: 'application/pdf' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${suggestedName}.pdf`;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    return;
  }
  await api.saveExportFile(bytes, 'pdf', suggestedName);
}

function extractLocalComparison(result: Json): {
  rmi: string | null;
  corrected: boolean;
  salarioBeneficio: string | null;
  coeficiente: string | null;
  regra: string | null;
} {
  const base = localRmi(result);
  const r = asObj(result);
  const envelope = asObj(r?.data ?? r?.result);
  const data = asObj(envelope?.native_result) ?? envelope;
  const summary = asObj(data?.resumo);
  const especie = asObj(data?.especie);
  const transicoes = asObj(data?.transicoes);
  const sbRaw = summary?.salario_beneficio;
  const coefRaw = summary?.coeficiente;
  const regra =
    asStr(transicoes?.regra_escolhida) ||
    asStr(especie?.regra_nome) ||
    asStr(especie?.regra_escolhida) ||
    null;
  return {
    ...base,
    salarioBeneficio: sbRaw !== undefined && sbRaw !== null && String(sbRaw).trim() !== '' ? String(sbRaw) : null,
    coeficiente:
      coefRaw !== undefined && coefRaw !== null && String(coefRaw).trim() !== ''
        ? String(coefRaw) + (String(coefRaw).includes('%') ? '' : '%')
        : null,
    regra,
  };
}

export function FabricaCheck({
  draft,
  workspace,
  onChange,
}: {
  draft: Calculation;
  workspace: Workspace;
  onChange: (next: Calculation) => void;
}) {
  const defaultEspecie = Number(draft.especie || '42');
  const defaultTipo: 'COMUM' | 'PCD' | 'RURICOLA' = ['Leve', 'Moderada', 'Grave'].includes(draft.disability ?? '')
    ? 'PCD'
    : 'COMUM';

  const [activeTab, setActiveTab] = useState<FabricaTab>('rmi');
  const [status, setStatus] = useState<FabricaStatus | null>(null);
  const [inlineKey, setInlineKey] = useState('');
  const [savingKey, setSavingKey] = useState(false);

  const [especie, setEspecie] = useState<number>(defaultEspecie);
  const [tipo, setTipo] = useState<'COMUM' | 'PCD' | 'RURICOLA'>(defaultTipo);
  const [fundamento, setFundamento] = useState('');
  const [demonstrativo, setDemonstrativo] = useState(false);
  const [permalink, setPermalink] = useState(false);

  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  // Tab 2: Tempo
  const [porGrupos, setPorGrupos] = useState(false);
  const [tempoDemo, setTempoDemo] = useState(false);
  const [tempoBusy, setTempoBusy] = useState(false);
  const [tempoError, setTempoError] = useState('');
  const [tempoResult, setTempoResult] = useState<Json | null>(null);

  // Tab 3: Atrasados
  const [atrasadosDib, setAtrasadosDib] = useState(() => isoOrEmpty(draft.dib));
  const [atrasadosDer, setAtrasadosDer] = useState(() => isoOrEmpty(draft.dib));
  const [atrasadosEspecie, setAtrasadosEspecie] = useState<number>(defaultEspecie);
  const [atrasadosRmi, setAtrasadosRmi] = useState('');
  const [atrasadosAtualizacao, setAtrasadosAtualizacao] = useState(() => new Date().toISOString().slice(0, 10));
  const [atrasadosNumero, setAtrasadosNumero] = useState('');
  const [atrasadosDemo, setAtrasadosDemo] = useState(false);
  const [atrasadosCustomJson, setAtrasadosCustomJson] = useState('');
  const [atrasadosBusy, setAtrasadosBusy] = useState(false);
  const [atrasadosError, setAtrasadosError] = useState('');
  const [atrasadosResult, setAtrasadosResult] = useState<Json | null>(null);

  // Tab 4: Documentos
  const [docType, setDocType] = useState<'cnis' | 'carta' | 'rdctc' | 'pap' | 'prevjud'>('cnis');
  const [docFile, setDocFile] = useState<File | null>(null);
  const [docBusy, setDocBusy] = useState(false);
  const [docError, setDocError] = useState('');
  const [docResult, setDocResult] = useState<Json | null>(null);

  // Tab 5: Índices e Tabelas
  const [indexType, setIndexType] = useState<string>(FABRICA_INDEX_TYPES[0].id);
  const [indexInicio, setIndexInicio] = useState('');
  const [indexFim, setIndexFim] = useState('');
  const [indexNome, setIndexNome] = useState('');
  const [indexBusy, setIndexBusy] = useState(false);
  const [indexError, setIndexError] = useState('');
  const [indexResult, setIndexResult] = useState<Json | null>(null);
  const [indexLabel, setIndexLabel] = useState('');

  useEffect(() => {
    setEspecie(defaultEspecie);
    setAtrasadosEspecie(defaultEspecie);
  }, [defaultEspecie, draft.id]);

  useEffect(() => {
    setTipo(defaultTipo);
  }, [defaultTipo, draft.id]);

  useEffect(() => {
    const dibIso = isoOrEmpty(draft.dib);
    if (dibIso) {
      setAtrasadosDib(dibIso);
      setAtrasadosDer(dibIso);
    }
  }, [draft.dib, draft.id]);

  // Crucial contract: NEVER call api.fabricaStatus / invoke on mount unless in Web mode.
  useEffect(() => {
    if (!WEB_MODE) return;
    let alive = true;
    api
      .fabricaStatus()
      .then(s => {
        if (alive) setStatus(s);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);

  const desktopKey = (workspace.settings.fabrica_api_key ?? '').trim();
  const hasKey = WEB_MODE ? Boolean(status?.configured) : Boolean(desktopKey || status?.vaultConfigured);
  const effectiveKey = desktopKey;

  const summary = (draft.fabrica ?? null) as FabricaSummary | null;
  const local = useMemo(() => extractLocalComparison(draft.result), [draft.result]);
  const atrasadosRmiEdited = useRef(false);
  const lastDraftIdForRmi = useRef<number | string>(draft.id);

  useEffect(() => {
    if (lastDraftIdForRmi.current !== draft.id) {
      lastDraftIdForRmi.current = draft.id;
      atrasadosRmiEdited.current = false;
      setAtrasadosRmi('');
    }
  }, [draft.id]);

  useEffect(() => {
    if (atrasadosRmiEdited.current) return;
    const candidate =
      summary?.provider?.maiorRmi && summary.provider.maiorRmi !== '0.00'
        ? summary.provider.maiorRmi
        : local.rmi ?? '';
    if (candidate) setAtrasadosRmi(candidate);
  }, [summary?.provider?.maiorRmi, local.rmi]);

  const hasCustomRmiOverrides = Boolean(
    especie !== defaultEspecie || tipo !== defaultTipo || demonstrativo || permalink || fundamento.trim(),
  );

  const activeBuilt = useMemo(() => {
    const reviewed = buildFabricaInput(draft);
    if (reviewed.input && !hasCustomRmiOverrides) {
      return { input: reviewed.input, issues: [], sourceMode: 'reviewed' as const };
    }
    return buildFabricaInputDirect(draft, {
      ...(especie !== defaultEspecie ? { especie } : {}),
      ...(tipo !== defaultTipo ? { tipo } : {}),
      ...(demonstrativo ? { demonstrativo: true } : {}),
      ...(permalink ? { permalink: true } : {}),
      ...(fundamento.trim() ? { fundamento: fundamento.trim() } : {}),
    });
  }, [draft, especie, tipo, demonstrativo, permalink, fundamento, defaultEspecie, defaultTipo, hasCustomRmiOverrides]);

  const stale = Boolean(
    summary?.inputFingerprint && activeBuilt.input && summary.inputFingerprint !== JSON.stringify(activeBuilt.input),
  );

  async function saveInlineWebKey() {
    const trimmed = inlineKey.trim();
    if (!trimmed) return;
    setSavingKey(true);
    setError('');
    try {
      await api.fabricaSaveKey(trimmed);
      setInlineKey('');
      const s = await api.fabricaStatus();
      setStatus(s);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setSavingKey(false);
    }
  }

  async function sendRmi() {
    if (!activeBuilt.input || !hasKey) return;
    setBusy(true);
    setError('');
    try {
      const hasOptions = demonstrativo || permalink || Boolean(fundamento.trim());
      const result = hasOptions
        ? await api.conferirFabricaRmi(activeBuilt.input, effectiveKey, {
            demonstrativo,
            permalink,
            fundamento: Boolean(fundamento.trim()),
          })
        : await api.conferirFabricaRmi(activeBuilt.input, effectiveKey);
      onChange({
        ...draft,
        fabrica: { ...result, inputFingerprint: JSON.stringify(activeBuilt.input) } as unknown as Calculation['fabrica'],
      });
      setConfirm(false);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }

  async function runTempo() {
    const builtTempo = activeBuilt.input
      ? activeBuilt
      : buildFabricaInputDirect(draft, { especie, tipo, demonstrativo: tempoDemo });
    if (!builtTempo.input || !hasKey) return;
    setTempoBusy(true);
    setTempoError('');
    try {
      const res = await api.calcularFabricaTempo(builtTempo.input as unknown as Json, effectiveKey || undefined, {
        porGrupos,
        demonstrativo: tempoDemo,
        permalink: false,
      });
      setTempoResult(res);
    } catch (reason) {
      setTempoError(errorMessage(reason));
    } finally {
      setTempoBusy(false);
    }
  }

  async function runAtrasados() {
    if (!hasKey) return;
    setAtrasadosBusy(true);
    setAtrasadosError('');
    try {
      let body: Record<string, unknown>;
      if (atrasadosCustomJson.trim()) {
        body = JSON.parse(atrasadosCustomJson) as Record<string, unknown>;
      } else {
        const rmiParsed = parseRmiInput(atrasadosRmi);
        body = {
          dadosProcesso: {
            numero: atrasadosNumero.trim() || '0000000-00.0000.0.00.0000',
            protocolo: atrasadosDer || atrasadosDib || atrasadosAtualizacao,
          },
          opcoesCalculo: {
            dataAtualizacao: atrasadosAtualizacao,
            consectariosPrincipal: {},
            consectariosAcessorios: {},
            criterioDeducaoAlcada: '',
            deduzirExcedenteAlcada: false,
          },
          beneficiosDevidos: [
            {
              especie: atrasadosEspecie,
              dib: atrasadosDib,
              der: atrasadosDer || atrasadosDib,
              ...(rmiParsed !== null ? { rmi: rmiParsed } : {}),
            },
          ],
          beneficiosPagos: [],
          periodosBeneficioVantajoso: [],
          descontos: [],
          acrescimos: [],
          outroDebitos: [],
          honorarios: {},
          multa: {},
        };
      }
      const res = await api.calcularFabricaAtrasados(body as unknown as Json, effectiveKey || undefined, {
        demonstrativo: atrasadosDemo,
        permalink: false,
      });
      setAtrasadosResult(res);
    } catch (reason) {
      setAtrasadosError(errorMessage(reason));
    } finally {
      setAtrasadosBusy(false);
    }
  }

  async function runDocumento() {
    if (!hasKey) return;
    setDocBusy(true);
    setDocError('');
    try {
      const res = await api.lerDocumentoFabrica(docType, effectiveKey || undefined, docFile ?? undefined);
      if (res !== null) setDocResult(res);
    } catch (reason) {
      setDocError(errorMessage(reason));
    } finally {
      setDocBusy(false);
    }
  }

  async function runIndices(endpoint: 'indice' | 'fundamentos' | 'marcos-temporais', label: string) {
    setIndexBusy(true);
    setIndexError('');
    setIndexLabel(label);
    try {
      const query: Record<string, string> = {};
      if (endpoint === 'indice') {
        if (indexInicio.trim()) query.competenciaInicial = indexInicio.trim();
        if (indexFim.trim()) query.competenciaFinal = indexFim.trim();
        if (indexNome.trim() && indexType === 'tabela') query.indice = indexNome.trim();
      }
      const res = await api.consultarFabricaIndices(
        endpoint,
        endpoint === 'indice' ? indexType : undefined,
        Object.keys(query).length ? query : undefined,
        effectiveKey || undefined,
      );
      setIndexResult(res);
    } catch (reason) {
      setIndexError(errorMessage(reason));
    } finally {
      setIndexBusy(false);
    }
  }

  return (
    <section id="fabrica-hub" className="fabrica-hub" aria-label="Central Fábrica de Cálculos TRF3">
      <header className="fabrica-hub__head">
        <div>
          <h2>Conferência na Fábrica de Cálculos/TRF3</h2>
          <p className="card__hint">
            Integração completa com a API oficial TRF3: RMI (17 espécies), tempo, atrasados, leitor documental e índices.
          </p>
        </div>
        <div className="head-actions">
          <Badge tone={activeBuilt.sourceMode === 'reviewed' ? 'good' : 'review'}>
            {activeBuilt.sourceMode === 'reviewed' ? 'Entrada revisada' : 'Entrada direta (CNIS/vínculos)'}
          </Badge>
          <Badge tone={hasKey ? 'good' : 'review'}>{hasKey ? 'Chave ativa' : 'Sem chave'}</Badge>
        </div>
      </header>

      <div className="fabrica-tabs" role="tablist" aria-label="Serviços da Fábrica TRF3">
        {FABRICA_TABS.map(tab => (
          <button
            key={tab.id}
            type="button"
            role="tab"
            id={`fabrica-tab-${tab.id}`}
            aria-selected={activeTab === tab.id}
            aria-controls={`fabrica-panel-${tab.id}`}
            className={'fabrica-tab' + (activeTab === tab.id ? ' active' : '')}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {!hasKey && activeTab !== 'indices' && (
        <div>
          <Notice>
            Informe a chave da API em <strong>Configurações → Fábrica de Cálculos</strong>.{' '}
            {WEB_MODE
              ? 'Na versão web a chave fica somente na sessão (sessionStorage) e nunca é salva no banco de dados.'
              : 'A chave fica na base local cifrada.'}
          </Notice>
          {WEB_MODE && (
            <div className="form-grid" style={{ marginTop: 12, alignItems: 'end' }}>
              <Field
                type="password"
                label="Chave TRF3 nesta sessão"
                value={inlineKey}
                onChange={setInlineKey}
                placeholder="Cole sua x-api-key TRF3"
              />
              <div className="head-actions">
                <Button primary disabled={savingKey || !inlineKey.trim()} onClick={() => void saveInlineWebKey()}>
                  {savingKey ? 'Salvando...' : 'Salvar chave na sessão'}
                </Button>
              </div>
            </div>
          )}
        </div>
      )}

      {activeTab === 'rmi' && (
        <div role="tabpanel" id="fabrica-panel-rmi" aria-labelledby="fabrica-tab-rmi">
          <p className="form-note" style={{ marginTop: 0 }}>
            A conferência envia somente data de nascimento, sexo, DIB, espécie e os períodos/salários revisados. Não envia
            nome, CPF, NIT, CNIS, PDF, anotações ou a chave dentro do corpo. O resultado do provedor fica registrado com
            fonte, horário e hash.
          </p>

          <div className="form-grid" data-ui-stack style={{ marginTop: 14 }}>
            <label className="field">
              <span>Espécie (17 espécies TRF3)</span>
              <select
                className="input"
                aria-label="Espécie TRF3"
                value={especie}
                onChange={e => setEspecie(Number(e.target.value))}
              >
                {FABRICA_ESPECIES.map(sp => (
                  <option key={sp.code} value={sp.code}>
                    {sp.label}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span>Modalidade / tipo de segurado</span>
              <select
                className="input"
                aria-label="Tipo de segurado TRF3"
                value={tipo}
                onChange={e => setTipo(e.target.value as 'COMUM' | 'PCD' | 'RURICOLA')}
              >
                <option value="COMUM">Comum (RGPS urbano/geral)</option>
                <option value="PCD">Pessoa com Deficiência (LC 142/2013)</option>
                <option value="RURICOLA">Rurícola / Segurado Especial</option>
              </select>
            </label>
            <Field
              label="Fundamento específico (opcional)"
              value={fundamento}
              onChange={setFundamento}
              placeholder="Ex.: EC_103_ART_20 (vazio = melhor benefício)"
            />
            <div className="field" style={{ justifyContent: 'flex-end', gap: 10 }}>
              <label className="checkline">
                <input type="checkbox" checked={demonstrativo} onChange={e => setDemonstrativo(e.target.checked)} />
                Gerar PDF Demonstrativo Oficial TRF3
              </label>
              <label className="checkline">
                <input type="checkbox" checked={permalink} onChange={e => setPermalink(e.target.checked)} />
                Gerar link permanente (permalink)
              </label>
            </div>
          </div>

          {activeBuilt.issues.length > 0 && (
            <Notice>
              <strong>Antes de enviar:</strong>
              <ul className="review-list">
                {activeBuilt.issues.slice(0, 8).map(issue => (
                  <li key={issue}>{issue}</li>
                ))}
              </ul>
            </Notice>
          )}
          {error && <Notice tone="error">{error}</Notice>}

          {!summary ? (
            <div className="form-actions" data-ui-actions>
              <Button primary disabled={busy || !hasKey || !activeBuilt.input} onClick={() => setConfirm(true)}>
                {busy ? (
                  <>
                    <span className="spinner" />
                    Conferindo...
                  </>
                ) : (
                  'Conferir na Fábrica de Cálculos'
                )}
              </Button>
              <span className="form-note">
                {activeBuilt.input
                  ? `${activeBuilt.input.periodos.length} período(s) e ${activeBuilt.input.periodos.reduce((total, period) => total + period.salarios.length, 0)} salário(s) preparados.`
                  : 'Resolva as pendências acima.'}
              </span>
            </div>
          ) : (
            <ProviderSummary
              summary={summary}
              local={local}
              stale={stale}
              busy={busy}
              error={error}
              hasKey={hasKey}
              canSend={Boolean(activeBuilt.input)}
              onAgain={() => setConfirm(true)}
              onClear={() => onChange({ ...draft, fabrica: null })}
            />
          )}
        </div>
      )}

      {activeTab === 'tempo' && (
        <div role="tabpanel" id="fabrica-panel-tempo" aria-labelledby="fabrica-tab-tempo">
          <p className="form-note" style={{ marginTop: 0 }}>
            Apura tempo simples, convertido, carência e elegibilidade às regras diretamente na Fábrica TRF3 usando os períodos
            do caso.
          </p>
          <div className="head-actions" style={{ marginTop: 12 }}>
            <label className="checkline">
              <input type="checkbox" checked={porGrupos} onChange={e => setPorGrupos(e.target.checked)} />
              Calcular por grupos de contribuição
            </label>
            <label className="checkline">
              <input type="checkbox" checked={tempoDemo} onChange={e => setTempoDemo(e.target.checked)} />
              Gerar PDF Demonstrativo Oficial
            </label>
          </div>
          {activeBuilt.issues.length > 0 && (
            <Notice>
              <strong>Pendências dos períodos:</strong>
              <ul className="review-list">
                {activeBuilt.issues.slice(0, 6).map(issue => (
                  <li key={issue}>{issue}</li>
                ))}
              </ul>
            </Notice>
          )}
          {tempoError && <Notice tone="error">{tempoError}</Notice>}
          <div className="form-actions" data-ui-actions>
            <Button primary disabled={tempoBusy || !hasKey || !activeBuilt.input} onClick={() => void runTempo()}>
              {tempoBusy ? (
                <>
                  <span className="spinner" />
                  Calculando tempo...
                </>
              ) : (
                'Calcular Tempo na Fábrica TRF3'
              )}
            </Button>
          </div>
          {tempoResult && <TempoResultView data={tempoResult} />}
        </div>
      )}

      {activeTab === 'atrasados' && (
        <div role="tabpanel" id="fabrica-panel-atrasados" aria-labelledby="fabrica-tab-atrasados">
          <p className="form-note" style={{ marginTop: 0 }}>
            Liquidação de parcelas vencidas e vincendas com parâmetros pré-preenchidos a partir do caso atual.
          </p>
          <div className="form-grid" data-ui-stack style={{ marginTop: 12 }}>
            <Field label="DIB (AAAA-MM-DD)" value={atrasadosDib} onChange={setAtrasadosDib} placeholder="2026-01-15" />
            <Field
              label="DER / Protocolo (AAAA-MM-DD)"
              value={atrasadosDer}
              onChange={setAtrasadosDer}
              placeholder="2026-01-15"
            />
            <label className="field">
              <span>Espécie do benefício</span>
              <select
                className="input"
                aria-label="Espécie para atrasados"
                value={atrasadosEspecie}
                onChange={e => setAtrasadosEspecie(Number(e.target.value))}
              >
                {FABRICA_ESPECIES.map(sp => (
                  <option key={sp.code} value={sp.code}>
                    {sp.label}
                  </option>
                ))}
              </select>
            </label>
            <Field
              label="RMI inicial (R$)"
              value={atrasadosRmi}
              onChange={v => {
                atrasadosRmiEdited.current = true;
                setAtrasadosRmi(v);
              }}
              placeholder="Ex.: 2500.00 ou 2.500,00"
            />
            <Field
              label="Data da atualização (AAAA-MM-DD)"
              value={atrasadosAtualizacao}
              onChange={setAtrasadosAtualizacao}
              placeholder="2026-11-18"
            />
            <Field
              label="Número do processo (opcional)"
              value={atrasadosNumero}
              onChange={setAtrasadosNumero}
              placeholder="0000000-00.0000.0.00.0000"
            />
          </div>
          <div className="head-actions" style={{ marginTop: 12 }}>
            <label className="checkline">
              <input type="checkbox" checked={atrasadosDemo} onChange={e => setAtrasadosDemo(e.target.checked)} />
              Gerar PDF Demonstrativo de Liquidação
            </label>
          </div>
          <details className="details" style={{ marginTop: 12 }}>
            <summary>Personalizar JSON completo de atrasados (opcional)</summary>
            <textarea
              className="text-area mono"
              aria-label="JSON personalizado de atrasados"
              style={{ minHeight: 180 }}
              value={atrasadosCustomJson}
              onChange={e => setAtrasadosCustomJson(e.target.value)}
              placeholder="Deixe em branco para usar o formulário acima, ou cole o payload completo de /calculos-atrasados."
            />
          </details>
          {atrasadosError && <Notice tone="error">{atrasadosError}</Notice>}
          <div className="form-actions" data-ui-actions>
            <Button primary disabled={atrasadosBusy || !hasKey} onClick={() => void runAtrasados()}>
              {atrasadosBusy ? (
                <>
                  <span className="spinner" />
                  Liquidando atrasados...
                </>
              ) : (
                'Calcular Atrasados / Liquidação'
              )}
            </Button>
          </div>
          {atrasadosResult && <AtrasadosResultView data={atrasadosResult} />}
        </div>
      )}

      {activeTab === 'documentos' && (
        <div role="tabpanel" id="fabrica-panel-documentos" aria-labelledby="fabrica-tab-documentos">
          <p className="form-note" style={{ marginTop: 0 }}>
            Envia um PDF selecionado explicitamente por você para o extrator oficial da Fábrica TRF3 (CNIS, Carta, RDCTC, PAP ou
            PREVJUD).
          </p>
          <div className="form-grid" data-ui-stack style={{ marginTop: 12 }}>
            <label className="field">
              <span>Tipo de documento INSS / Judicial</span>
              <select
                className="input"
                aria-label="Tipo de documento TRF3"
                value={docType}
                onChange={e => setDocType(e.target.value as 'cnis' | 'carta' | 'rdctc' | 'pap' | 'prevjud')}
              >
                {FABRICA_DOC_TYPES.map(d => (
                  <option key={d.id} value={d.id}>
                    {d.label}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span>Arquivo PDF (opcional se quiser escolher ao clicar)</span>
              <input
                type="file"
                accept="application/pdf,.pdf"
                className="input"
                aria-label="Selecionar arquivo PDF documental"
                onChange={e => setDocFile(e.target.files?.[0] ?? null)}
              />
            </label>
          </div>
          {docError && <Notice tone="error">{docError}</Notice>}
          <div className="form-actions" data-ui-actions>
            <Button primary disabled={docBusy || !hasKey} onClick={() => void runDocumento()}>
              {docBusy ? (
                <>
                  <span className="spinner" />
                  Lendo documento...
                </>
              ) : (
                <>
                  <Icon name="upload" size={16} />
                  {docFile ? `Ler "${docFile.name}" na Fábrica TRF3` : 'Selecionar PDF e Ler na Fábrica TRF3'}
                </>
              )}
            </Button>
          </div>
          {docResult && (
            <div style={{ marginTop: 16 }}>
              <Notice tone="ok">Documento processado pelo leitor da Fábrica TRF3.</Notice>
              <details className="details" open>
                <summary>Dados extraídos do documento ({docType.toUpperCase()})</summary>
                <div className="result-json mono">{JSON.stringify(docResult, null, 2)}</div>
              </details>
            </div>
          )}
        </div>
      )}

      {activeTab === 'indices' && (
        <div role="tabpanel" id="fabrica-panel-indices" aria-labelledby="fabrica-tab-indices">
          <p className="form-note" style={{ marginTop: 0 }}>
            Consulte séries econômicas oficiais, fundamentos jurídicos e marcos temporais diretamente na API TRF3.
          </p>
          <div className="form-grid" data-ui-stack style={{ marginTop: 12 }}>
            <label className="field">
              <span>Série / tabela econômica</span>
              <select
                className="input"
                aria-label="Série ou tabela TRF3"
                value={indexType}
                onChange={e => setIndexType(e.target.value)}
              >
                {FABRICA_INDEX_TYPES.map(idx => (
                  <option key={idx.id} value={idx.id}>
                    {idx.label}
                  </option>
                ))}
              </select>
            </label>
            <Field
              label="Nome do índice (opcional para 'tabela')"
              value={indexNome}
              onChange={setIndexNome}
              placeholder="Ex.: INPC, IPCA, SELIC"
            />
            <Field
              label="Competência inicial (AAAA-MM)"
              value={indexInicio}
              onChange={setIndexInicio}
              placeholder="2020-01"
            />
            <Field
              label="Competência final (AAAA-MM)"
              value={indexFim}
              onChange={setIndexFim}
              placeholder="2026-01"
            />
          </div>
          {indexError && <Notice tone="error">{indexError}</Notice>}
          <div className="head-actions" data-ui-actions style={{ marginTop: 14 }}>
            <Button primary disabled={indexBusy} onClick={() => void runIndices('indice', indexType)}>
              {indexBusy ? 'Consultando...' : 'Consultar Índice / Tabela'}
            </Button>
            <Button disabled={indexBusy} onClick={() => void runIndices('fundamentos', 'Fundamentos legais')}>
              Consultar Fundamentos
            </Button>
            <Button disabled={indexBusy} onClick={() => void runIndices('marcos-temporais', 'Marcos temporais')}>
              Consultar Marcos Temporais
            </Button>
          </div>
          {indexResult && <IndicesResultView label={indexLabel} data={indexResult} />}
        </div>
      )}

      {confirm && activeBuilt.input && (
        <Modal
          title="Enviar dados revisados à Fábrica de Cálculos?"
          subtitle="Confira exatamente o que sai do computador. Nome, CPF, NIT, CNIS, PDF e anotações ficam fora."
          onClose={() => setConfirm(false)}
        >
          <div className="form-grid" data-ui-stack>
            <div>
              <span className="form-note">Espécie</span>
              <strong>{activeBuilt.input.especie}</strong>
            </div>
            <div>
              <span className="form-note">DIB / fato gerador</span>
              <strong>
                {activeBuilt.input.dataApuracao}
                {activeBuilt.input.fatoGerador ? ' · ' + activeBuilt.input.fatoGerador : ''}
              </strong>
            </div>
            <div>
              <span className="form-note">Períodos</span>
              <strong>{activeBuilt.input.periodos.length}</strong>
            </div>
            <div>
              <span className="form-note">Salários</span>
              <strong>{activeBuilt.input.periodos.reduce((total, period) => total + period.salarios.length, 0)}</strong>
            </div>
          </div>
          <p className="form-note">
            A chave vai apenas no cabeçalho <code>x-api-key</code> para o endpoint oficial. A resposta completa não é gravada; só
            o resumo, a fonte e o hash.
          </p>
          {error && <Notice tone="error">{error}</Notice>}
          <div className="form-actions" data-ui-actions>
            <Button onClick={() => setConfirm(false)}>Cancelar</Button>
            <Button primary disabled={busy} onClick={() => void sendRmi()}>
              {busy ? (
                <>
                  <span className="spinner" />
                  Enviando...
                </>
              ) : (
                'Enviar e conferir'
              )}
            </Button>
          </div>
        </Modal>
      )}
    </section>
  );
}

function ProviderSummary({
  summary,
  local,
  stale,
  busy,
  error,
  hasKey,
  canSend,
  onAgain,
  onClear,
}: {
  summary: FabricaSummary;
  local: {
    rmi: string | null;
    corrected: boolean;
    salarioBeneficio: string | null;
    coeficiente: string | null;
    regra: string | null;
  };
  stale: boolean;
  busy: boolean;
  error: string;
  hasKey: boolean;
  canSend: boolean;
  onAgain: () => void;
  onClear: () => void;
}) {
  const provider = summary.provider;
  const best = provider.beneficios?.[0];
  const providerRmi = provider.maiorRmi;
  const hasRmi = providerRmi !== '0.00' && Boolean(best);
  const diff = hasRmi ? difference(local.rmi, providerRmi) : null;
  const demoPdf = typeof provider.demonstrativoPdf === 'string' && provider.demonstrativoPdf.length > 0 ? provider.demonstrativoPdf : null;
  const safePermalink =
    typeof provider.permalink === 'string' && provider.permalink.startsWith('https://') ? provider.permalink : null;

  return (
    <div style={{ marginTop: 18 }}>
      <div className="head-actions" data-ui-actions style={{ marginBottom: 12 }}>
        <Badge tone="review">Provedor externo</Badge>
        <Badge>Fonte e hash registrados</Badge>
      </div>
      {stale && (
        <Notice tone="error">
          Os dados do caso mudaram depois desta conferência. Execute novamente para comparar a mesma hipótese.
        </Notice>
      )}
      {error && <Notice tone="error">{error}</Notice>}
      {!hasRmi && (
        <Notice>
          <strong>A Fábrica não apurou RMI para esta hipótese.</strong> O provedor pode exigir uma declaração de
          DII/incapacidade ou um requisito que não foi enviado. O motor local e a revisão continuam valendo; nenhum valor foi
          inventado.
        </Notice>
      )}
      {best?.alerta && <Notice>{best.alerta}</Notice>}
      {provider.conclusao?.completo === false && (
        <Notice>
          A Fábrica interrompeu o cálculo
          {provider.conclusao.motivoInterrupcao ? ': ' + provider.conclusao.motivoInterrupcao : '.'} O valor exibido é o que
          ela retornou.
        </Notice>
      )}

      <div className="comparison-grid" aria-label="Comparação lado a lado entre Motor Local e Fábrica TRF3">
        <div className="comparison-card">
          <span>Motor Local (Rust)</span>
          <strong>{local.rmi ? amount(local.rmi) : 'Não calculada'}</strong>
          <dl>
            <dt>Salário de Benefício</dt>
            <dd>{local.salarioBeneficio ? amount(local.salarioBeneficio) : '—'}</dd>
            <dt>Coeficiente</dt>
            <dd>{local.coeficiente ?? '—'}</dd>
            <dt>Regra</dt>
            <dd>{local.regra ?? '—'}</dd>
            <dt>Correção local</dt>
            <dd>{local.rmi ? (local.corrected ? 'Ativa' : 'Nominal / sem correção') : '—'}</dd>
          </dl>
        </div>

        <div className="comparison-card comparison-card--accent">
          <span>Fábrica de Cálculos / TRF3</span>
          <strong>{hasRmi ? amount(providerRmi) : 'Não apurada'}</strong>
          <dl>
            <dt>Salário de Benefício</dt>
            <dd>{amount(best?.dadosRmi?.salarioBeneficio)}</dd>
            <dt>Coeficiente</dt>
            <dd>{coefficient(best?.dadosRmi?.coeficiente)}</dd>
            <dt>Regra / Fundamento</dt>
            <dd>{best?.fundamento ?? best?.chaveFundamento ?? '—'}</dd>
            <dt>Diferença (TRF3 vs Local)</dt>
            <dd>{diff ? `${deltaMoney(diff.cents)} (${percent(diff.percent)})` : '—'}</dd>
          </dl>
        </div>
      </div>

      {(provider.beneficios?.length ?? 0) > 0 && (
        <div className="table-wrap" style={{ marginTop: 14 }}>
          <table className="data-table">
            <thead>
              <tr>
                <th>Benefício</th>
                <th>Fundamento</th>
                <th className="num">RMI</th>
                <th className="num">SB</th>
                <th className="num">Coef.</th>
                <th>Competências</th>
              </tr>
            </thead>
            <tbody>
              {provider.beneficios.map((benefit, index) => (
                <tr key={index}>
                  <td>
                    <strong>{benefit.descricao ?? 'Benefício'}</strong>
                    <div className="secondary-line">espécie {benefit.especie ?? '—'}</div>
                  </td>
                  <td>
                    {benefit.fundamento ?? benefit.chaveFundamento ?? '—'}
                    <div className="secondary-line">{benefit.alerta ?? ''}</div>
                  </td>
                  <td className="num">{amount(benefit.dadosRmi?.rmi)}</td>
                  <td className="num">{amount(benefit.dadosRmi?.salarioBeneficio)}</td>
                  <td className="num">{coefficient(benefit.dadosRmi?.coeficiente)}</td>
                  <td>
                    {benefit.dadosRmi?.competenciaInicial ?? '—'} a {benefit.dadosRmi?.competenciaFinal ?? '—'}
                    <div className="secondary-line">
                      divisor {benefit.dadosRmi?.divisor ?? '—'} · {benefit.dadosRmi?.totalCompetenciasConsideradas ?? '—'}{' '}
                      competências
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {(best?.requisitos?.length ?? 0) > 0 && (
        <div className="table-wrap" style={{ marginTop: 14 }}>
          <table className="data-table">
            <thead>
              <tr>
                <th>Requisito</th>
                <th>Resultado do provedor</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {best?.requisitos?.map((requisito, index) => (
                <tr key={index}>
                  <td>{requisito.rotulo ?? requisito.nome}</td>
                  <td>
                    {requisito.porExtenso ?? `${String(requisito.exigido ?? '—')} / ${String(requisito.apurado ?? '—')}`}
                    {requisito.nota ? <div className="secondary-line">{requisito.nota}</div> : null}
                  </td>
                  <td>
                    {requisito.cumprido === true ? 'Cumprido' : requisito.cumprido === false ? 'Não cumprido' : '—'}
                    {requisito.inconsistencia ? ' · inconsistência' : ''}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {local.rmi && !local.corrected && (
        <Notice>
          O resultado local está sem correção monetária; a Fábrica corrige os salários. Ative a correção no editor e execute o
          motor local para uma comparação direta.
        </Notice>
      )}
      {local.rmi && local.corrected && !diff && hasRmi && (
        <Notice>
          Não foi possível comparar os valores; confira os centavos na memória dos dois resultados.
        </Notice>
      )}

      <details className="details">
        <summary>Fonte e proveniência</summary>
        <ul className="review-list">
          <li>Endpoint oficial: {summary.source_url}</li>
          <li>Consultado em: {when(summary.retrieved_at)}</li>
          <li>Hash da resposta: {shortHash(summary.response_sha256)}</li>
          <li>Hash do pedido: {shortHash(summary.request_sha256)}</li>
          <li>
            Tamanho: {(summary.bytes / 1024 / 1024).toFixed(1)} MB · {provider.resultados ?? 0} resultado(s) ·{' '}
            {provider.linhasDescartes ?? 0} linha(s) de descarte
          </li>
          <li>Conclusão do provedor: {provider.conclusao?.completo === false ? 'interrompida' : 'completa'}</li>
        </ul>
      </details>

      <Notice>
        <strong>Limite:</strong> a Fábrica é um provedor externo. O valor depende dos dados enviados e das regras dela; não é
        certificação de direito nem concessão. O motor local continua experimental e separado. A chave não aparece neste resumo.
      </Notice>

      <div className="form-actions" data-ui-actions>
        {demoPdf && (
          <Button onClick={() => void downloadBase64Pdf(demoPdf, 'trf3-demonstrativo-rmi')}>
            <Icon name="file" size={16} />
            Baixar PDF Demonstrativo TRF3
          </Button>
        )}
        {safePermalink && (
          <a className="btn btn-ghost" href={safePermalink} target="_blank" rel="noopener noreferrer">
            Abrir permalink TRF3
          </a>
        )}
        <Button primary disabled={busy || !hasKey || !canSend} onClick={onAgain}>
          {busy ? (
            <>
              <span className="spinner" />
              Conferindo...
            </>
          ) : (
            'Conferir novamente'
          )}
        </Button>
        <Button disabled={busy} onClick={onClear}>
          Limpar conferência
        </Button>
        <span className="form-note">A conferência fica no caso; salve o cálculo para guardar a fonte e o hash.</span>
      </div>
    </div>
  );
}

function TempoResultView({ data }: { data: Json }) {
  const root = asObj(data);
  const res = asObj(root?.result);
  const totais = asObj(res?.totais);
  const beneficios = asObj(res?.beneficios);
  const cumpridos = asArr(beneficios?.cumpridos).map(asObj).filter((x): x is Record<string, unknown> => x !== null);
  const naoCumpridos = asArr(beneficios?.naoCumpridos).map(asObj).filter((x): x is Record<string, unknown> => x !== null);
  const periodos = asArr(res?.periodos ?? res?.grupos).map(asObj).filter((x): x is Record<string, unknown> => x !== null);
  const demoPdf = typeof root?.demonstrativoPdf === 'string' && root.demonstrativoPdf.length > 0 ? root.demonstrativoPdf : null;

  return (
    <div style={{ marginTop: 16 }}>
      {totais && (
        <div className="result-strip">
          <div>
            <span>Tempo simples</span>
            <strong>{formatDaysBreakdown(totais.simples)}</strong>
          </div>
          <div>
            <span>Tempo convertido</span>
            <strong>{formatDaysBreakdown(totais.convertido)}</strong>
          </div>
          <div>
            <span>Carência</span>
            <strong>{totais.carencia != null ? `${asStr(totais.carencia)} meses` : '—'}</strong>
          </div>
          <div>
            <span>Idade apurada</span>
            <strong>{totais.idade != null ? `${asStr(totais.idade)} anos` : '—'}</strong>
          </div>
        </div>
      )}
      {demoPdf && (
        <div className="head-actions" style={{ marginBottom: 12 }}>
          <Button onClick={() => void downloadBase64Pdf(demoPdf, 'trf3-demonstrativo-tempo')}>
            <Icon name="file" size={16} />
            Baixar PDF Demonstrativo de Tempo
          </Button>
        </div>
      )}
      {cumpridos.length > 0 && (
        <div className="table-wrap" style={{ marginTop: 10 }}>
          <table className="data-table">
            <thead>
              <tr>
                <th>Benefício cumprido</th>
                <th>Fundamento</th>
                <th>Situação</th>
              </tr>
            </thead>
            <tbody>
              {cumpridos.map((b, i) => (
                <tr key={i}>
                  <td><strong>{asStr(b.descricao) || 'Benefício'}</strong></td>
                  <td>{asStr(b.fundamento ?? b.chaveFundamento) || '—'}</td>
                  <td><Badge tone="good">Cumprido</Badge></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {naoCumpridos.length > 0 && (
        <div className="table-wrap" style={{ marginTop: 10 }}>
          <table className="data-table">
            <thead>
              <tr>
                <th>Benefício analisado</th>
                <th>Fundamento</th>
                <th>Situação</th>
              </tr>
            </thead>
            <tbody>
              {naoCumpridos.map((b, i) => (
                <tr key={i}>
                  <td>{asStr(b.descricao) || 'Benefício'}</td>
                  <td>{asStr(b.fundamento ?? b.chaveFundamento) || '—'}</td>
                  <td><Badge tone="neutral">Não cumprido</Badge></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {periodos.length > 0 && (
        <details className="details" style={{ marginTop: 10 }}>
          <summary>Períodos / grupos processados ({periodos.length})</summary>
          <div className="result-json mono">{JSON.stringify(periodos, null, 2)}</div>
        </details>
      )}
      <details className="details" style={{ marginTop: 10 }}>
        <summary>JSON completo de tempo TRF3</summary>
        <div className="result-json mono">{JSON.stringify(data, null, 2)}</div>
      </details>
    </div>
  );
}

function AtrasadosResultView({ data }: { data: Json }) {
  const root = asObj(data);
  const res = asObj(root?.result) ?? root;
  const demoPdf = typeof root?.demonstrativoPdf === 'string' && root.demonstrativoPdf.length > 0 ? root.demonstrativoPdf : null;

  const totalGeral = res?.totalGeral ?? res?.totalDevido ?? res?.valorTotal ?? res?.total;
  const totalPrincipal = res?.totalPrincipal ?? res?.principalCorrigido ?? res?.principal;
  const totalJuros = res?.totalJuros ?? res?.juros;
  const totalCorrecao = res?.totalCorrecao ?? res?.correcaoMonetaria;

  return (
    <div style={{ marginTop: 16 }}>
      <div className="result-strip">
        <div>
          <span>Total apurado</span>
          <strong>{totalGeral != null ? amount(String(totalGeral)) : 'Ver detalhes abaixo'}</strong>
        </div>
        {totalPrincipal != null && (
          <div>
            <span>Principal</span>
            <strong>{amount(String(totalPrincipal))}</strong>
          </div>
        )}
        {totalCorrecao != null && (
          <div>
            <span>Correção monetária</span>
            <strong>{amount(String(totalCorrecao))}</strong>
          </div>
        )}
        {totalJuros != null && (
          <div>
            <span>Juros</span>
            <strong>{amount(String(totalJuros))}</strong>
          </div>
        )}
      </div>
      {demoPdf && (
        <div className="head-actions" style={{ marginBottom: 12 }}>
          <Button onClick={() => void downloadBase64Pdf(demoPdf, 'trf3-demonstrativo-atrasados')}>
            <Icon name="file" size={16} />
            Baixar PDF Demonstrativo de Atrasados
          </Button>
        </div>
      )}
      <details className="details" open>
        <summary>Demonstrativo completo de atrasados TRF3</summary>
        <div className="result-json mono">{JSON.stringify(data, null, 2)}</div>
      </details>
    </div>
  );
}

function IndicesResultView({ label, data }: { label: string; data: Json }) {
  const root = asObj(data);
  const rowsRaw = Array.isArray(data) ? data : asArr(root?.result);
  const rows = rowsRaw.map(asObj).filter((x): x is Record<string, unknown> => x !== null);
  const cols = rows.length ? Object.keys(rows[0]).slice(0, 6) : [];

  return (
    <div style={{ marginTop: 16 }}>
      {rows.length > 0 && cols.length > 0 ? (
        <div className="table-wrap">
          <table className="data-table">
            <caption>{label || 'Resultado da consulta TRF3'} ({rows.length} registros)</caption>
            <thead>
              <tr>
                {cols.map(c => (
                  <th key={c}>{c}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.slice(0, 150).map((row, idx) => (
                <tr key={idx}>
                  {cols.map(c => (
                    <td key={c}>
                      {row[c] == null
                        ? '—'
                        : typeof row[c] === 'object'
                          ? JSON.stringify(row[c])
                          : String(row[c])}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <details className="details" open>
          <summary>{label || 'Resposta TRF3'}</summary>
          <div className="result-json mono">{JSON.stringify(data, null, 2)}</div>
        </details>
      )}
    </div>
  );
}
