import { lazy, Suspense, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { api, errorMessage, type FabricaStatus } from './api';
import { repararJson } from './web-api';
import { checkForUpdates } from './updater';
import type { Workspace, Client, Calculation, Appointment, Template, ReferenceTable } from './model';
import {
  blankWorkspace,
  blankClient,
  blankCase,
  allocate,
  allocateStringId,
  putEntity,
  blockers,
  KINDS,
  REFERENCE_LABELS,
  brDate,
  localDate,
  isoDate,
  prepareCaseForSave,
  blankInterview,
  interviewOf,
  hasFinancialResult,
} from './model';
import { Badge, Button, Empty, Field, Icon, Modal, Notice, Panel, Select } from './components/ui';
import { CaseWorkbench } from './components/CaseWorkbench';
import { RobotAssistant, RobotFace, assistantResume } from './components/RobotAssistant';
import { ReportCenter } from './components/ReportCenter';
import { AtriumImportModal, AtriumSendModal } from './components/AtriumSync';
import { trackPage } from './web-telemetry';
import type { CaseDocument } from './components/MinutaPage';
import { MotionProvider, PageTransition, NavIndicator, useMotionPolicy } from './components/amicro/motion';
import { DashboardOverview } from './components/amicro/DashboardOverview';
import { AppSkeleton } from './components/lucian/AppSkeleton';
import { LucianToast } from './components/lucian/LucianToast';
import { LucianEmpty } from './components/lucian/LucianEmpty';
import { revealTheme, SunMoon } from './components/lucian/ThemeReveal';
import { flushSync } from 'react-dom';

// Páginas pouco usadas carregam sob demanda para a primeira abertura ficar mais leve.
const ToolsPage = lazy(() => import('./components/ToolsPage').then(m => ({ default: m.ToolsPage })));
const CoverageMap = lazy(() => import('./components/CoverageMap').then(m => ({ default: m.CoverageMap })));
const AiCopilot = lazy(() => import('./components/AiCopilot').then(m => ({ default: m.AiCopilot })));
const KnowledgePage = lazy(() => import('./components/KnowledgePage').then(m => ({ default: m.KnowledgePage })));
const MinutaPage = lazy(() => import('./components/MinutaPage').then(m => ({ default: m.MinutaPage })));
const pageFallback = <p className="form-note" role="status">Carregando a página...</p>;

type Page =
  | 'home'
  | 'clients'
  | 'cases'
  | 'rgps'
  | 'robot'
  | 'ai'
  | 'knowledge'
  | 'minuta'
  | 'wizard'
  | 'tables'
  | 'tools'
  | 'documents'
  | 'reports'
  | 'agenda'
  | 'settings'
  | 'coverage';

const NAV: { id: Page; label: string; full?: string; icon: string; group: string }[] = [
  { id: 'home', label: 'Início', icon: 'home', group: 'Início' },
  { id: 'cases', label: 'Cálculos', icon: 'calc', group: 'Cálculos' },
  { id: 'robot', label: 'Assistente de cálculo', icon: 'file', group: 'Cálculos' },
  { id: 'wizard', label: 'Cálculo previdenciário', icon: 'file', group: 'Mesa do cálculo' },
  { id: 'reports', label: 'Relatórios', icon: 'file', group: 'Relatórios PDF' },
  { id: 'tables', label: 'Tabelas e índices', icon: 'chart', group: 'Ferramentas e índices' },
  { id: 'tools', label: 'Ferramentas', icon: 'tool', group: 'Ferramentas e índices' },
  { id: 'clients', label: 'Clientes', icon: 'users', group: 'Clientes e configurações' },
  { id: 'settings', label: 'Configurações', icon: 'settings', group: 'Clientes e configurações' },
  { id: 'rgps', label: 'Meus cálculos', icon: 'calc', group: 'Mais' },
  { id: 'ai', label: 'Copiloto IA', icon: 'spark', group: 'Mais' },
  { id: 'minuta', label: 'Minuta assistida', icon: 'file', group: 'Mais' },
  { id: 'knowledge', label: 'Base de conhecimento', icon: 'library', group: 'Mais' },
  { id: 'documents', label: 'Documentos', icon: 'book', group: 'Mais' },
  { id: 'agenda', label: 'Agenda', icon: 'calendar', group: 'Mais' },
  { id: 'coverage', label: 'Cobertura e validação', icon: 'shield', group: 'Mais' },
];

/** Áreas do menu lateral. As páginas do grupo "Mais" ficam no botão "Mais páginas". */
const NAV_GROUPS = ['Início', 'Cálculos', 'Mesa do cálculo', 'Relatórios PDF', 'Ferramentas e índices', 'Clientes e configurações'] as const;

const WEB_MODE = import.meta.env.MODE === 'web';

const DESCRIPTIONS: Record<Page, string> = {
  home: '',
  clients: 'Cadastro local, sem dados de demonstração.',
  cases: '',
  rgps: '',
  robot: '',
  ai: 'Envia ao provedor apenas o que você autorizar.',
  knowledge: 'Consulta local, sem IA.',
  minuta: '',
  wizard: '',
  tables: 'Nenhum coeficiente oficial é inventado.',
  tools: '',
  documents: 'Modelos locais com campos de substituição.',
  reports: WEB_MODE
    ? 'Relatórios gerados no servidor da sua conta. O cálculo é enviado ao servidor autorizado e os PDFs salvos expiram.'
    : 'Dados, memória e pendências em arquivos locais, sem envio a terceiros.',
  agenda: '',
  settings: '',
  coverage: 'O que foi observado com REA, implementado e efetivamente testado.',
};

function createDemoCase(): Calculation {
  const base = blankCase();
  const salaries: Calculation['manual_salaries'] = [];
  let salaryId = 100;
  for (let year = 1995; year <= 2025; year += 1) {
    for (let month = 1; month <= 12; month += 1) {
      const competence = `${year}-${String(month).padStart(2, '0')}`;
      const amount = year < 2005 ? '2800.00' : year < 2015 ? '3600.00' : '4500.00';
      salaries.push({ id: salaryId++, competence, amount, indicators: '', confirmed: true });
    }
  }
  return {
    ...base,
    id: 0,
    name: 'Caso Demonstrativo RGPS (Sintético)',
    kind: 'RGPS',
    status: 'Dados conferidos; pronto para simular',
    identity: {
      full_name: 'Segurado Demonstrativo Fictício',
      cpf: null,
      nit: null,
      birth_date: '1961-05-15',
      mother_name: null,
    },
    sex: 'Masculino',
    disability: 'Sem deficiência declarada',
    dib: '15/01/2026',
    scenario: 'Simulação de novo benefício',
    especie: '42',
    manual_links: [
      {
        id: 1,
        employer: 'Empregador Demonstrativo A (Sintético)',
        start_date: '1995-01-01',
        end_date: '2009-12-31',
        regime: 'RGPS',
        indicators: '',
        confirmed: true,
      },
      {
        id: 2,
        employer: 'Empregador Demonstrativo B (Sintético)',
        start_date: '2010-01-01',
        end_date: '2025-12-31',
        regime: 'RGPS',
        indicators: '',
        confirmed: true,
      },
    ],
    manual_salaries: salaries,
    options: {
      ...base.options,
      correction_index: 'INPC',
      rule_policy: JSON.stringify({
        version: 'rgps-core-observational-v1',
        review_evidence: 'Caso demonstrativo sintético carregado pelo usuário',
        manual_link_kind: 'empregado',
        manual_currency: 'BRL',
        native_options: {
          corrigirSalarios: true,
          descarte: false,
          divisorMinimo: true,
          aplicarTetoVigenteDer: true,
          referenciaCorrecao: '2026-01',
          expectativaSobrevida: null,
          regraAlvo: null,
          incapacidadeOrigem: 'comum',
          grauEspecial: null,
          tempoEspecialMeses: null,
          especialAntesReforma: null,
          magisterioMeses: null,
          magisterioExclusivo: null,
          magisterioAntesReforma: null,
        },
        exclude_imported_source: false,
        source_exclusion_evidence: null,
        identity_confirmed: true,
      }),
    },
    notes: 'Caso demonstrativo sintético gerado apenas sob demanda para testes e demonstração.',
  };
}

function WorkspaceApp() {
  const motionPolicy = useMotionPolicy();
  const [w, setW] = useState<Workspace>(blankWorkspace);
  const [ready, setReady] = useState(false);
  const [storageReady, setStorageReady] = useState(false);
  const [page, setPage] = useState<Page>('home');
  useEffect(() => {
    if (WEB_MODE) trackPage(page);
  }, [page]);
  const [toast, setToast] = useState<{ message: string; error: boolean } | null>(null);
  const [formError, setFormError] = useState<{ message: string; field: string | null } | null>(null);
  const [saving, setSaving] = useState(false);
  const [search, setSearch] = useState('');
  const [archived, setArchived] = useState(false);
  const [filterKind, setFilterKind] = useState('Todos os módulos');
  const [selected, setSelected] = useState<number | null>(null);
  const [draft, setDraft] = useState<Calculation>(blankCase);
  const [client, setClient] = useState<Client | null>(null);
  const [atriumImportOpen, setAtriumImportOpen] = useState(false);
  const [atriumSendClient, setAtriumSendClient] = useState<Client | null>(null);
  const [appointment, setAppointment] = useState<Appointment | null>(null);
  const [template, setTemplate] = useState<Template | null>(null);
  const [table, setTable] = useState<ReferenceTable | null>(null);
  const [palette, setPalette] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState('');
  const [caseTab, setCaseTab] = useState(0);
  const [workbenchKey, setWorkbenchKey] = useState(0);
  const [creationChoice, setCreationChoice] = useState<string | null>(null);
  const [manualSettings, setManualSettings] = useState<Workspace['settings'] | null>(null);
  const [mobileMenu, setMobileMenu] = useState(false);
  const [routeMotion, setRouteMotion] = useState('none');
  // Lucian UX/UI Prompt #417 (MIT, https://lucianlabs.dev/uxui-prompt/#417): destaque do menu salta com mola até o item ativo.
  const sideNav = useRef<HTMLElement>(null);
  const sidePill = useRef<HTMLSpanElement>(null);
  const sideFirst = useRef(true);
  const [mobileLayout, setMobileLayout] = useState(
    () => typeof window.matchMedia === 'function' && window.matchMedia('(max-width:1023px)').matches,
  );
  useLayoutEffect(() => {
    const nav = sideNav.current, pill = sidePill.current;
    if (!nav || !pill) return;
    const btn = nav.querySelector<HTMLElement>('.side-link.active');
    pill.style.opacity = btn ? '1' : '0';
    if (!btn) return;
    // Mede só quando a página ativa muda: medir a cada render forçava o navegador a recalcular o layout.
    let top = 0;
    for (let el: HTMLElement | null = btn; el && el !== nav; el = el.offsetParent as HTMLElement | null) top += el.offsetTop;
    if (sideFirst.current) pill.style.transition = 'none';
    pill.style.setProperty('--py', top + 'px');
    pill.style.height = btn.offsetHeight + 'px';
    if (sideFirst.current) { void pill.offsetWidth; pill.style.transition = ''; sideFirst.current = false; }
  }, [page, mobileLayout]);
  const [fabricaStatus, setFabricaStatus] = useState<FabricaStatus | null>(null);
  const [fabricaStatusBusy, setFabricaStatusBusy] = useState(false);
  const [fabricaStatusError, setFabricaStatusError] = useState('');
  const [webFabricaKey, setWebFabricaKey] = useState('');

  useEffect(() => {
    if (typeof window.matchMedia !== 'function') return;
    const media = window.matchMedia('(max-width:1023px)');
    const update = () => {
      setMobileLayout(media.matches);
      if (!media.matches) setMobileMenu(false);
    };
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);

  const inputMode = useRef('pointer');
  const mainRef = useRef<HTMLElement>(null);
  const lastFocusPage = useRef<Page>('home');

  useEffect(() => {
    const pointer = () => {
      inputMode.current = 'pointer';
      document.documentElement.dataset.input = 'pointer';
    };
    const keyboard = () => {
      inputMode.current = 'keyboard';
      document.documentElement.dataset.input = 'keyboard';
    };
    document.addEventListener('pointerdown', pointer, true);
    document.addEventListener('keydown', keyboard, true);
    return () => {
      document.removeEventListener('pointerdown', pointer, true);
      document.removeEventListener('keydown', keyboard, true);
    };
  }, []);

  useEffect(() => {
    if (ready && mainRef.current) {
      const changed = lastFocusPage.current !== page;
      lastFocusPage.current = page;
      mainRef.current.scrollTop = 0;
      if (changed) mainRef.current.querySelector<HTMLElement>('.page-head h1')?.focus({ preventScroll: true });
    }
  }, [page, ready]);

  function notify(message: string, error = false) {
    setToast({ message, error });
  }
  function failForm(message: string, field: string | null = null) {
    setFormError({ message, field });
  }
  function fieldError(field: string) {
    return formError?.field === field ? formError.message : undefined;
  }

  useEffect(() => {
    if (!formError) return;
    requestAnimationFrame(() => document.querySelector<HTMLElement>('.dialog [aria-invalid="true"]')?.focus());
  }, [formError]);

  useEffect(() => {
    let live = true;
    api
      .boot()
      .then(b => {
        if (live) {
          const workspace = repararJson(b.workspace);
          setW(workspace);
          setStorageReady(b.storage_ready);
          if (b.message) notify(b.message, !b.storage_ready);
          setReady(true);
        }
      })
      .catch(() => {
        if (live) {
          notify(
            'A interface precisa do aplicativo desktop para acessar a base local. A inicialização falhou; gravações estão bloqueadas.',
            true,
          );
          setReady(true);
        }
      });
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    if (!WEB_MODE || page !== 'settings') return;
    let live = true;
    setFabricaStatusBusy(true);
    setFabricaStatusError('');
    api
      .fabricaStatus()
      .then(s => {
        if (live) setFabricaStatus(s);
      })
      .catch(e => {
        if (live) setFabricaStatusError(errorMessage(e));
      })
      .finally(() => {
        if (live) setFabricaStatusBusy(false);
      });
    return () => {
      live = false;
    };
  }, [page]);

  useEffect(() => {
    document.documentElement.dataset.theme = w.settings.light_theme ? 'light' : 'dark';
  }, [w.settings.light_theme]);

  useEffect(() => {
    if (toast && !toast.error) {
      const id = setTimeout(() => setToast(null), 6000);
      return () => clearTimeout(id);
    }
  }, [toast]);

  useEffect(() => {
    function key(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setPalette(v => !v);
      }
      if (e.key === 'Escape') setPalette(false);
    }
    document.addEventListener('keydown', key);
    return () => document.removeEventListener('keydown', key);
  }, []);

  async function commit(next: Workspace, quiet = false) {
    if (!storageReady) {
      notify('A base não está disponível com segurança. Nenhum arquivo será sobrescrito.', true);
      throw new Error('Base indisponível');
    }
    setSaving(true);
    try {
      const sanitized = WEB_MODE
        ? { ...next, settings: { ...next.settings, fabrica_api_key: '' } }
        : next;
      const saved = await api.save(sanitized);
      setW(saved);
      if (!quiet) notify('Alterações salvas na base local protegida.');
    } catch (e) {
      notify(errorMessage(e), true);
      throw e;
    } finally {
      setSaving(false);
    }
  }

  function go(p: Page) {
    if (p !== page) setRouteMotion(inputMode.current);
    setPage(p);
    setSearch('');
    setPalette(false);
    setMobileMenu(false);
  }
  function start(kind = 'RGPS') {
    setCreationChoice(kind);
  }
  function begin(kind: string, assisted: boolean) {
    setDraft({ ...blankCase(), kind, assistant: { ...blankInterview(), mode: assisted ? 'assisted' : 'manual' } });
    setSelected(null);
    setCaseTab(0);
    setWorkbenchKey(k => k + 1);
    setCreationChoice(null);
    go(assisted ? 'robot' : 'wizard');
  }
  function openCase(c: Calculation) {
    setDraft({ ...structuredClone(c), assistant: c.assistant ?? null });
    setSelected(c.id);
    setCaseTab(0);
    setWorkbenchKey(k => k + 1);
    go(c.assistant?.mode === 'assisted' ? 'robot' : 'wizard');
  }
  function loadDemoCase() {
    setDraft(createDemoCase());
    setSelected(null);
    setCaseTab(7);
    setWorkbenchKey(k => k + 1);
    go('wizard');
    notify('Caso demonstrativo sintético carregado no editor. Execute o motor local ou consulte a Fábrica TRF3.');
  }

  async function saveCase(c: Calculation, options?: { quiet?: boolean }) {
    if (!c.name.trim()) {
      if (!options?.quiet) notify('Informe um nome para o cálculo.', true);
      return;
    }
    const next = structuredClone(w);
    const blocked = (c.result as Record<string, unknown> | null)?.status === 'blocked';
    const saved = {
      ...prepareCaseForSave(next, c),
      status: blocked
        ? 'Execução bloqueada; revisar dados'
        : hasFinancialResult(c)
          ? 'Simulação técnica calculada'
          : blockers(c).length
            ? 'Dados pendentes'
            : 'Dados conferidos; execução pendente',
    };
    next.calculations = putEntity(next.calculations, saved);
    next.next_id = allocate(next);
    try {
      await commit(next, options?.quiet);
      if (!options?.quiet) {
        setDraft(saved);
        setSelected(saved.id);
      }
    } catch {
      /* already shown, keep original state */
    }
  }

  function updateCaseSettings(c: Calculation) {
    const next = structuredClone(w);
    next.calculations = putEntity(next.calculations, c);
    next.next_id = allocate(next);
    void commit(next).catch(() => {});
  }

  async function saveClient() {
    if (!client) return;
    if (!client.name.trim()) {
      failForm('Informe o nome do cliente.', 'name');
      return;
    }
    if (client.birth_date && !isoDate(client.birth_date)) {
      failForm('Informe uma data de nascimento válida (DD/MM/AAAA).', 'birth');
      return;
    }
    const next = structuredClone(w);
    const value = { ...client, id: client.id || allocate(next) };
    next.clients = putEntity(next.clients, value);
    next.next_id = allocate(next);
    try {
      await commit(next);
      setFormError(null);
      setClient(null);
    } catch {
      /* already shown */
    }
  }

  async function archiveClient(c: Client) {
    const next = structuredClone(w);
    next.clients = next.clients.map(x => (x.id === c.id ? { ...x, archived: !x.archived } : x));
    try {
      await commit(next);
    } catch {
      /* already shown */
    }
  }

  async function archiveCase(c: Calculation) {
    const next = structuredClone(w);
    next.calculations = next.calculations.map(x => (x.id === c.id ? { ...x, archived: !x.archived } : x));
    try {
      await commit(next);
    } catch {
      /* already shown */
    }
  }

  async function duplicate(c: Calculation) {
    const next = structuredClone(w);
    let id = allocate(next);
    const copy = {
      ...structuredClone(c),
      id: id++,
      name: c.name + ', cópia',
      status: 'Rascunho duplicado',
      result: null,
      manual_links: c.manual_links.map(row => ({ ...structuredClone(row), id: id++ })),
      manual_salaries: c.manual_salaries.map(row => ({ ...structuredClone(row), id: id++ })),
    };
    next.calculations.push(copy);
    next.next_id = id;
    try {
      await commit(next);
      openCase(copy);
    } catch {
      /* already shown */
    }
  }

  async function saveAppointment() {
    if (!appointment) return;
    if (!appointment.title.trim()) {
      failForm('Informe o título do compromisso.', 'title');
      return;
    }
    if (!isoDate(appointment.date)) {
      failForm('Informe uma data válida (DD/MM/AAAA).', 'date');
      return;
    }
    if (!/^([01]\d|2[0-3]):[0-5]\d$/.test(appointment.time)) {
      failForm('Informe a hora no formato HH:MM.', 'time');
      return;
    }
    const next = structuredClone(w);
    const a = { ...appointment, id: appointment.id || allocate(next) };
    next.appointments = putEntity(next.appointments, a);
    next.next_id = allocate(next);
    try {
      await commit(next);
      setFormError(null);
      setAppointment(null);
    } catch {
      /* already shown */
    }
  }

  async function saveTemplate() {
    if (!template) return;
    if (!template.name.trim()) {
      failForm('Informe o título do modelo.', 'name');
      return;
    }
    if (!template.body.trim()) {
      failForm('Informe o conteúdo do modelo.', 'body');
      return;
    }
    const next = structuredClone(w);
    const d = { ...template, id: template.id || allocate(next) };
    next.documents = putEntity(next.documents, d);
    next.next_id = allocate(next);
    try {
      await commit(next);
      setFormError(null);
      setTemplate(null);
    } catch {
      /* already shown */
    }
  }

  async function saveMinutaTemplate(name: string, body: string) {
    if (!name.trim() || !body.trim()) {
      notify('Informe nome e conteúdo da minuta.', true);
      throw new Error('Minuta vazia');
    }
    const next = structuredClone(w);
    const id = allocate(next);
    next.documents = putEntity(next.documents, { id, name: name.trim(), category: 'Minuta assistida', body });
    next.next_id = allocate(next);
    await commit(next);
  }

  async function saveMinutaReading(document: CaseDocument, caseId: number | null) {
    if (!caseId) throw new Error('Vincule um caso antes de salvar a leitura.');
    if (!document.hashSha256) throw new Error('O anexo não possui hash SHA-256; extraia novamente.');
    const next = structuredClone(w);
    let source = next.source_documents.find(item => item.hash_sha256 === document.hashSha256);
    if (!source) {
      const id = allocateStringId(next, 'doc');
      source = {
        id,
        nome_arquivo: document.fileName,
        mime: 'application/pdf',
        paginas: document.pages ?? 0,
        origem_texto: document.usedOcr ? 'ocr' : 'nativo',
        ocr_confianca: null,
        hash_sha256: document.hashSha256,
        criado_em: new Date().toISOString(),
      };
      next.source_documents.push(source);
    }
    let reading = next.readings.find(item => item.documento_id === source.id && item.tipo === 'documento_generico');
    if (!reading) {
      const id = allocateStringId(next, 'read');
      reading = {
        id,
        documento_id: source.id,
        tipo: 'documento_generico',
        campos: [],
        paginas_lidas: document.pages ?? 0,
        pendencias: [],
        status: 'confirmada',
        texto_extraido: document.text,
      };
      next.readings.push(reading);
    } else {
      reading.status = 'confirmada';
      reading.texto_extraido = document.text;
    }
    const calc = next.calculations.find(item => item.id === caseId);
    if (!calc) throw new Error('Caso não encontrado no workspace.');
    if (!calc.reading_ids.includes(reading.id)) calc.reading_ids.push(reading.id);
    next.next_id = allocate(next);
    await commit(next);
  }

  async function saveTable() {
    if (!table) return;
    if (!table.name.trim()) {
      failForm('Informe o nome da tabela.', 'name');
      return;
    }
    if (!table.provenance.trim()) {
      failForm('Informe a fonte ou versão da tabela.', 'provenance');
      return;
    }
    const next = structuredClone(w);
    const t = { ...table, id: table.id || allocate(next) };
    next.reference_tables = putEntity(next.reference_tables, t);
    next.next_id = allocate(next);
    try {
      await commit(next);
      setFormError(null);
      setTable(null);
    } catch {
      /* already shown */
    }
  }

  async function addLabels() {
    const next = structuredClone(w);
    for (const label of REFERENCE_LABELS) {
      if (!next.reference_tables.some(t => t.name === label)) {
        const id = allocate(next);
        next.reference_tables.push({
          id,
          name: label,
          unit: 'A confirmar',
          periodicity: 'A confirmar',
          entries: [],
          provenance: 'Rótulo observado; sem valores importados',
          verified: false,
        });
        next.next_id = id + 1;
      }
    }
    try {
      await commit(next);
    } catch {
      /* shown */
    }
  }

  async function testFabricaConnection() {
    setFabricaStatusBusy(true);
    setFabricaStatusError('');
    try {
      const s = await api.fabricaStatus();
      setFabricaStatus(s);
      notify(s.configured ? 'Conexão com a Fábrica TRF3 configurada.' : 'Fábrica TRF3 sem chave ativa neste ambiente.', !s.configured);
    } catch (e) {
      const msg = errorMessage(e);
      setFabricaStatusError(msg);
      notify(msg, true);
    } finally {
      setFabricaStatusBusy(false);
    }
  }

  async function saveFabricaKeyFromSettings() {
    const targetKey = WEB_MODE ? webFabricaKey.trim() : ((manualSettings ?? w.settings).fabrica_api_key ?? '').trim();
    if (!targetKey) {
      notify('Informe uma chave da API da Fábrica TRF3 para salvar.', true);
      return;
    }
    setFabricaStatusBusy(true);
    setFabricaStatusError('');
    try {
      if (WEB_MODE) {
        await api.fabricaSaveKey(targetKey);
        setWebFabricaKey('');
        const s = await api.fabricaStatus();
        setFabricaStatus(s);
        notify('Chave da Fábrica TRF3 salva apenas nesta sessão (sessionStorage).');
      } else {
        const nextSettings = { ...(manualSettings ?? w.settings), fabrica_api_key: targetKey };
        await commit({ ...w, settings: nextSettings }, true);
        setManualSettings(null);
        await api.fabricaSaveKey(targetKey);
        const s = await api.fabricaStatus();
        setFabricaStatus(s);
        notify('Chave da Fábrica TRF3 salva na base local protegida.');
      }
    } catch (e) {
      const msg = errorMessage(e);
      setFabricaStatusError(msg);
      notify(msg, true);
    } finally {
      setFabricaStatusBusy(false);
    }
  }

  async function clearFabricaKeyFromSettings() {
    setFabricaStatusBusy(true);
    setFabricaStatusError('');
    try {
      if (WEB_MODE) {
        await api.fabricaClearKey();
        setWebFabricaKey('');
        const s = await api.fabricaStatus();
        setFabricaStatus(s);
        notify('Chave da sessão TRF3 removida.');
      } else {
        const nextSettings = { ...(manualSettings ?? w.settings), fabrica_api_key: '' };
        await commit({ ...w, settings: nextSettings }, true);
        setManualSettings(null);
        await api.fabricaClearKey().catch(() => {});
        const s = await api.fabricaStatus().catch(() => null);
        if (s) setFabricaStatus(s);
        notify('Chave da Fábrica TRF3 removida.');
      }
    } catch (e) {
      const msg = errorMessage(e);
      setFabricaStatusError(msg);
      notify(msg, true);
    } finally {
      setFabricaStatusBusy(false);
    }
  }

  async function saveAllSettings() {
    const base = manualSettings ?? w.settings;
    if (WEB_MODE) {
      const clean = { ...base, fabrica_api_key: '' };
      try {
        await commit({ ...w, settings: clean });
        setManualSettings(null);
        if (webFabricaKey.trim()) {
          await api.fabricaSaveKey(webFabricaKey.trim());
          setWebFabricaKey('');
          const s = await api.fabricaStatus().catch(() => null);
          if (s) setFabricaStatus(s);
        }
      } catch {
        /* already shown */
      }
      return;
    }
    const trimmedKey = (base.fabrica_api_key ?? '').trim();
    const prevKey = (w.settings.fabrica_api_key ?? '').trim();
    try {
      await commit({ ...w, settings: base });
      setManualSettings(null);
      if (trimmedKey && trimmedKey !== prevKey) {
        await api.fabricaSaveKey(trimmedKey).catch(() => {});
      } else if (!trimmedKey && prevKey) {
        await api.fabricaClearKey().catch(() => {});
      }
    } catch {
      /* already shown */
    }
  }

  const selectedCase = w.calculations.find(c => c.id === selected) ?? null;
  const openLoop =
    w.calculations
      .filter(c => !c.archived && c.id > 0 && c.assistant?.mode === 'assisted' && !hasFinancialResult(c))
      .slice(-1)[0] ?? null;
  const resume = openLoop ? assistantResume(openLoop) : null;
  const clientRows = w.clients.filter(
    c =>
      (archived || !c.archived) &&
      [c.name, c.cpf, c.email, c.phone].join(' ').toLocaleLowerCase().includes(search.toLocaleLowerCase()),
  );
  const caseRows = w.calculations.filter(
    c =>
      (page !== 'rgps' || c.kind === 'RGPS') &&
      (archived || !c.archived) &&
      (filterKind === 'Todos os módulos' || c.kind === filterKind) &&
      [c.name, c.kind, c.status].join(' ').toLocaleLowerCase().includes(search.toLocaleLowerCase()),
  );

  if (!ready) {
    return (
      <AppSkeleton message="Abrindo a base local protegida..." />
    );
  }

  const activeSettings = manualSettings ?? w.settings;

  return (
    <div className="previdenciar-app" data-od-id="previdenciar-shell">
      <a
        className="skip-link"
        aria-label="Ir para o conteúdo"
        title="Ir para o conteúdo"
        href="#main-content"
        data-od-id="skip-to-content"
        onClick={e => {
          e.preventDefault();
          mainRef.current?.focus({ preventScroll: true });
        }}
      >
        <Icon name="arrow" size={18} />
        <span>Conteúdo</span>
      </a>

      <aside className="app-side" hidden={mobileLayout} data-od-id="desktop-navigation">
        <div className="brand-link">
          <span className="brand-name">previdenciar.io</span>
        </div>
        <nav ref={sideNav} className="side-nav u417-nav-host" aria-label="Navegação principal">
          <span ref={sidePill} className="u417-pl" aria-hidden="true" />
          {NAV_GROUPS.map(group => (
            <div className="side-group" key={group}>
              {group !== 'Início' && <div className="side-group-label">{group}</div>}
              {NAV.filter(n => n.group === group).map(n => (
                <div key={n.id}>
                  <button
                    type="button"
                    className={'side-link ' + (page === n.id ? 'active' : '')}
                    aria-current={page === n.id ? 'page' : undefined}
                    title={n.full ?? n.label}
                    aria-label={n.full ?? n.label}
                    data-od-id={'nav-' + n.id}
                    onClick={() => go(n.id)}
                  >
                    <Icon name={n.icon} />
                    <span className="side-label">{n.label}</span>
                  </button>
                </div>
              ))}
            </div>
          ))}
          <div className="side-group">
            <button
              type="button"
              className={'side-link ' + (NAV.some(n => n.group === 'Mais' && n.id === page) ? 'active' : '')}
              aria-label="Mais páginas"
              title="Mais páginas"
              data-od-id="nav-more"
              onClick={() => setMobileMenu(true)}
            >
              <Icon name="menu" />
              <span className="side-label">Mais páginas</span>
            </button>
          </div>
        </nav>
        <div className="side-foot">
          <Icon name="shield" size={18} />
          <span>
            Dados locais protegidos
            <br />
            Motor Rust · versão {__APP_VERSION__}
          </span>
        </div>
      </aside>

      <header className="top-chrome" data-od-id="office-toolbar">
        <span className="chrome-mobile-brand">
          <img src="/brand-icon.png" alt="" aria-hidden="true" />
          <span>previdenciar.io</span>
        </span>
        <div className="breadcrumb">
          Escritório
          <Icon name="chevron" size={13} />
          <strong>{NAV.find(n => n.id === page)?.full ?? NAV.find(n => n.id === page)?.label}</strong>
        </div>
        <span className="sr-only" role="status">
          {saving ? 'Salvando...' : storageReady ? 'Base local' : 'Base indisponível'}
        </span>
        <div className="chrome-actions" data-ui-actions>
          <Button
            primary
            className="chrome-new"
            aria-label="Novo cálculo no escritório"
            title="Novo cálculo no escritório"
            onClick={() => start()}
          >
            <Icon name="plus" size={17} />
            <span>Novo</span>
          </Button>
          <Button
            className="btn-icon"
            aria-label="Buscar no escritório"
            title="Buscar no escritório"
            onClick={() => setPalette(true)}
          >
            <Icon name="search" size={18} />
          </Button>
          <Button
            aria-label="Alternar tema"
            aria-pressed={w.settings.light_theme}
            title={w.settings.light_theme ? 'Ativar tema escuro' : 'Ativar tema claro'}
            onClick={e => {
              const light = !w.settings.light_theme;
              revealTheme(e.currentTarget, () => {
                document.documentElement.dataset.theme = light ? 'light' : 'dark';
                flushSync(() => setW({ ...w, settings: { ...w.settings, light_theme: light } }));
              });
            }}
          >
            <SunMoon dark={!w.settings.light_theme} />
          </Button>
        </div>
      </header>

      <main ref={mainRef} id="main-content" tabIndex={-1} className="app-content" data-od-id="main-workspace">
        {toast && !toast.error && <ToastMessage toast={toast} onClose={() => setToast(null)} />}
        <PageTransition className="page-wrap" key={page} pointer={routeMotion === 'pointer'} page={page}>
          <header className="page-head">
            <div>
              <h1 tabIndex={-1} data-od-id={'heading-' + page}>
                {NAV.find(n => n.id === page)?.full ?? NAV.find(n => n.id === page)?.label}
              </h1>
              {DESCRIPTIONS[page] && <p>{DESCRIPTIONS[page]}</p>}
            </div>
            <div className="head-actions" data-ui-actions>
              {page === 'home' ? (
                <>
                  <Button
                    primary
                    onClick={() => {
                      setDraft({ ...blankCase(), kind: 'RGPS', assistant: { ...blankInterview(), mode: 'manual' } });
                      setSelected(null);
                      setCaseTab(1);
                      setWorkbenchKey(k => k + 1);
                      go('wizard');
                    }}
                  >
                    <Icon name="upload" size={16} />
                    Importar CNIS (Cálculo Rápido)
                  </Button>
                  <Button onClick={loadDemoCase}>
                    <Icon name="spark" size={16} />
                    Carregar caso demonstrativo
                  </Button>
                </>
              ) : (page === 'cases' || page === 'rgps') && caseRows.length > 0 ? (
                <Button primary onClick={() => start()}>
                  <Icon name="plus" size={17} />
                  Novo cálculo
                </Button>
              ) : page === 'clients' && (clientRows.length > 0 || api.atrium) ? (
                <>
                  {api.atrium && (
                    <Button disabled={!storageReady} onClick={() => setAtriumImportOpen(true)}>
                      <Icon name="upload" size={16} />
                      Importar do Atrium
                    </Button>
                  )}
                  {clientRows.length > 0 && (
                    <Button primary onClick={() => setClient(blankClient())}>
                      <Icon name="plus" size={17} />
                      Novo cliente
                    </Button>
                  )}
                </>
              ) : null}
            </div>
          </header>

          {!storageReady && (
            <Notice tone="error">
              A base local não pôde ser aberta com segurança. Cadastros e gravações estão bloqueados; o arquivo existente não
              será sobrescrito.
            </Notice>
          )}

          {page === 'home' && openLoop && resume && (
            <Panel
              title="Retomar revisão"
              actions={<Badge tone="review">Em aberto</Badge>}
            >
              <div className="work-item">
                <span className="work-icon">
                  <Icon name="calc" />
                </span>
                <div className="work-copy">
                  <strong>{openLoop.name}</strong>
                  <p>
                    Você parou em “{resume.label}” · {resume.completed} de {resume.total} etapas registradas. Quando quiser,
                    continue exatamente daqui.
                  </p>
                </div>
                <Button primary onClick={() => openCase(openLoop)}>
                  Retomar de onde parei
                  <Icon name="arrow" size={16} />
                </Button>
              </div>
            </Panel>
          )}

          {page === 'home' && (
            <>
              <div className={'cockpit' + (w.appointments.filter(a => !a.completed).length ? '' : ' single')}>
                <Panel
                  title="Continue de onde parou"
                  actions={
                    w.calculations.filter(c => !c.archived).length ? (
                      <Button onClick={() => go('cases')}>
                        Ver cálculos
                        <Icon name="arrow" size={15} />
                      </Button>
                    ) : undefined
                  }
                >
                  {w.calculations.filter(c => !c.archived).length ? (
                    w.calculations
                      .filter(c => !c.archived)
                      .slice(-5)
                      .reverse()
                      .map(c => (
                        <div className="work-item" key={c.id}>
                          <span className="work-icon">
                            <Icon name="calc" />
                          </span>
                          <div className="work-copy">
                            <strong>{c.name}</strong>
                            <p>
                              {c.kind} · {c.scenario}
                            </p>
                          </div>
                          <Badge
                            tone={
                              hasFinancialResult(c)
                                ? 'good'
                                : (c.result as { status?: string } | null)?.status === 'blocked'
                                  ? 'error'
                                  : /Rascunho|Dados conferidos/.test(c.status)
                                    ? 'neutral'
                                    : 'review'
                            }
                          >
                            {hasFinancialResult(c)
                              ? 'Calculado'
                              : (c.result as { status?: string } | null)?.status === 'blocked'
                                ? 'Bloqueado'
                                : 'Revisão'}
                          </Badge>
                          <Button aria-label={'Abrir ' + c.name} onClick={() => openCase(c)}>
                            <Icon name="arrow" size={17} />
                          </Button>
                        </div>
                      ))
                  ) : (
                    <LucianEmpty
                      title="Seu escritório começa aqui"
                      doneTitle="Abrindo o novo cálculo"
                      description="Cadastre um cliente ou abra o assistente para organizar um cálculo. A base real não vem preenchida com casos fictícios."
                      actionLabel="Novo cálculo"
                      onAction={() => start()}
                    />
                  )}
                </Panel>
                <div className="cockpit-stack">
                  <Panel title="Próximos compromissos">
                    {w.appointments
                      .filter(a => !a.completed)
                      .slice(0, 3)
                      .map(a => (
                        <div className="work-item" key={a.id}>
                          <Icon name="calendar" size={18} />
                          <div className="work-copy">
                            <strong>{a.title}</strong>
                            <p>
                              {a.date} · {a.time}
                            </p>
                          </div>
                        </div>
                      ))}
                    {w.appointments.filter(a => !a.completed).length === 0 && (
                      <p className="secondary-line">Nenhum compromisso aberto.</p>
                    )}
                    {w.appointments.filter(a => !a.completed).length > 0 && (
                      <Button onClick={() => go('agenda')}>
                        Abrir agenda
                        <Icon name="arrow" size={15} />
                      </Button>
                    )}
                  </Panel>
                </div>
              </div>
              {w.calculations.some(c => !c.archived) && (
                <DashboardOverview
                  workspace={w}
                  onStart={() => start()}
                  onClient={() => setClient(blankClient())}
                  onReports={() => go('reports')}
                />
              )}
              <div className="quiet-banner">
                <Icon name="shield" />
                <p>
                  <strong>Resultado sujeito a revisão.</strong> Confira vínculos, salários e documentos antes de usar o cálculo.
                </p>
              </div>
            </>
          )}

          {page === 'clients' && (
            <>
              <div className="toolbar">
                <Search value={search} onChange={setSearch} placeholder="Pesquisar nome, CPF, e-mail ou telefone" />
                <label className="checkline">
                  <input type="checkbox" checked={archived} onChange={e => setArchived(e.target.checked)} />
                  Incluir arquivados
                </label>
                <span className="count">{clientRows.length} clientes</span>
              </div>
              {clientRows.length ? (
                <div className="table-wrap">
                  <table className="data-table">
                    <thead>
                      <tr>
                        <th>Cliente</th>
                        <th>CPF / CNPJ</th>
                        <th>Contato</th>
                        <th>Cálculos</th>
                        <th>Situação</th>
                        <th>Ações</th>
                      </tr>
                    </thead>
                    <tbody>
                      {clientRows.map(c => (
                        <tr key={c.id}>
                          <td>
                            <strong>{c.name}</strong>
                            <div className="secondary-line">{c.birth_date || 'Nascimento não informado'}</div>
                          </td>
                          <td>{c.cpf || 'Sem dado'}</td>
                          <td>
                            {c.email || 'Sem dado'}
                            <div className="secondary-line">{c.phone}</div>
                          </td>
                          <td>{w.calculations.filter(x => x.client_id === c.id && !x.archived).length}</td>
                          <td>
                            <Badge tone={c.archived ? 'neutral' : 'good'}>{c.archived ? 'Arquivado' : 'Ativo'}</Badge>
                          </td>
                          <td>
                            <div className="row-actions" data-ui-actions>
                              <Button onClick={() => setClient(structuredClone(c))}>Editar</Button>
                              <Button
                                onClick={() => {
                                  setDraft({
                                    ...blankCase(),
                                    client_id: c.id,
                                    identity: {
                                      ...blankCase().identity,
                                      full_name: c.name,
                                      cpf: c.cpf || null,
                                      birth_date: c.birth_date ? isoDate(c.birth_date) : null,
                                    },
                                    sex: c.sex,
                                  });
                                  setSelected(null);
                                  setCaseTab(0);
                                  setWorkbenchKey(k => k + 1);
                                  go('wizard');
                                }}
                              >
                                Novo cálculo
                              </Button>
                              {api.atrium && !c.archived && (
                                <Button disabled={!storageReady} onClick={() => setAtriumSendClient(structuredClone(c))}>
                                  {c.atrium_id ? 'Atualizar no Atrium' : 'Enviar ao Atrium'}
                                </Button>
                              )}
                              <Button onClick={() => void archiveClient(c)}>
                                {c.archived ? 'Reativar' : 'Arquivar'}
                              </Button>
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <Panel title="Cadastro de clientes">
                  <Empty
                    title={search ? 'Nenhum cliente encontrado' : 'Nenhum cliente cadastrado'}
                    description={
                      api.atrium
                        ? 'Inclua um cliente ou importe os clientes do Atrium para vincular simulações, documentos e compromissos.'
                        : 'Inclua um cliente para vincular suas simulações, documentos e compromissos.'
                    }
                    action={
                      <Button primary onClick={() => setClient(blankClient())}>
                        Novo cliente
                      </Button>
                    }
                  />
                </Panel>
              )}
            </>
          )}

          {(page === 'cases' || page === 'rgps') && (
            <>
              <div className="toolbar">
                <Search value={search} onChange={setSearch} placeholder="Pesquisar nome, módulo ou situação" />
                <select
                  aria-label="Filtrar módulo"
                  className="input"
                  value={filterKind}
                  onChange={e => setFilterKind(e.target.value)}
                >
                  {['Todos os módulos', ...KINDS].map(x => (
                    <option key={x}>{x}</option>
                  ))}
                </select>
                <label className="checkline">
                  <input type="checkbox" checked={archived} onChange={e => setArchived(e.target.checked)} />
                  Arquivados
                </label>
                <span className="count">{caseRows.length} cálculos</span>
              </div>
              {caseRows.length ? (
                <div className="table-wrap">
                  <table className="data-table">
                    <thead>
                      <tr>
                        <th>Cálculo</th>
                        <th>Módulo</th>
                        <th>DIB</th>
                        <th>Situação</th>
                        <th>Ações</th>
                      </tr>
                    </thead>
                    <tbody>
                      {caseRows.map(c => (
                        <tr key={c.id}>
                          <td>
                            <strong>{c.name}</strong>
                            <div className="secondary-line">{c.identity.full_name ?? 'Identificação pendente'}</div>
                          </td>
                          <td>{c.kind}</td>
                          <td>{c.dib}</td>
                          <td>
                            <Badge
                              tone={
                                hasFinancialResult(c)
                                  ? 'good'
                                  : (c.result as { status?: string } | null)?.status === 'blocked'
                                    ? 'error'
                                    : /Rascunho|Dados conferidos/.test(c.status)
                                      ? 'neutral'
                                      : 'review'
                              }
                            >
                              {c.status}
                            </Badge>
                          </td>
                          <td>
                            <div className="row-actions">
                              <Button onClick={() => openCase(c)}>Abrir</Button>
                              <Button onClick={() => void duplicate(c)}>Duplicar</Button>
                              <Button
                                onClick={() => {
                                  setSelected(c.id);
                                  go('reports');
                                }}
                              >
                                Relatório
                              </Button>
                              <Button onClick={() => void archiveCase(c)}>
                                {c.archived ? 'Reativar' : 'Arquivar'}
                              </Button>
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <Panel title="Cálculos do escritório">
                  <Empty
                    title="Nenhum cálculo neste filtro"
                    description="Abra o assistente e registre uma hipótese separada, preservando documentos e casos anteriores."
                    action={
                      <Button primary onClick={() => start()}>
                        Novo cálculo
                      </Button>
                    }
                  />
                </Panel>
              )}
              {page === 'rgps' && (
                <div className="quiet-banner">
                  <RobotFace />
                  <p>
                    <strong>Quer preencher com ajuda?</strong> Clique em Novo cálculo e escolha Usar o assistente de cálculos. O
                    robô pergunta, importa o CNIS e conduz a conferência sem pular dados ausentes.
                  </p>
                </div>
              )}
              <div className="module-grid">
                {KINDS.filter(k => k !== 'RGPS' && page !== 'rgps').map(k => (
                  <Panel key={k} title={k} className="module-card">
                    <p>
                      Organize dados, parâmetros e memória deste módulo. As fórmulas com parâmetros explícitos ficam nos módulos de
                      apoio.
                    </p>
                    <Button onClick={() => start(k)}>
                      Abrir caso
                      <Icon name="arrow" size={15} />
                    </Button>
                  </Panel>
                ))}
              </div>
            </>
          )}

          {page === 'ai' && <Suspense fallback={pageFallback}><AiCopilot notify={notify} workspace={w} /></Suspense>}
          {page === 'knowledge' && <Suspense fallback={pageFallback}><KnowledgePage notify={notify} /></Suspense>}
          {page === 'minuta' && (
            <Suspense fallback={pageFallback}>
              <MinutaPage
                workspace={w}
                notify={notify}
                onOpenCopilot={() => go('ai')}
                onSaveTemplate={(name, body) => saveMinutaTemplate(name, body)}
                onSaveReading={saveMinutaReading}
              />
            </Suspense>
          )}
          {page === 'robot' && (
            <RobotAssistant
              key={(selected ?? 'robot-new') + '-' + workbenchKey}
              draft={draft}
              setDraft={setDraft}
              workspace={w}
              onSave={saveCase}
              onNotify={notify}
              onManual={() => {
                setDraft({ ...draft, assistant: { ...interviewOf(draft), mode: 'manual' } });
                go('wizard');
              }}
              onManualAt={step => {
                setCaseTab(step);
                setWorkbenchKey(k => k + 1);
                go('wizard');
              }}
              onNew={() => setCreationChoice('RGPS')}
            />
          )}
          {page === 'wizard' && (
            <CaseWorkbench
              key={(selected ?? 'new') + '-' + workbenchKey}
              draft={draft}
              setDraft={setDraft}
              workspace={w}
              onSave={saveCase}
              onNotify={notify}
              initialStep={caseTab}
            />
          )}
          {page === 'tools' && <Suspense fallback={pageFallback}><ToolsPage notify={notify} /></Suspense>}

          {page === 'tables' && (
            <>
              <div className="toolbar">
                <Search value={search} onChange={setSearch} placeholder="Pesquisar índice ou tabela" />
                <Button onClick={() => void addLabels()}>Cadastrar rótulos observados</Button>
                <Button
                  primary
                  onClick={() =>
                    setTable({ id: 0, name: '', unit: '', periodicity: 'Mensal', entries: [], provenance: '', verified: false })
                  }
                >
                  <Icon name="plus" size={16} />
                  Nova tabela
                </Button>
              </div>
              <div className="table-wrap">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>Série / índice</th>
                      <th>Unidade</th>
                      <th>Períodos</th>
                      <th>Origem e revisão</th>
                      <th>Ação</th>
                    </tr>
                  </thead>
                  <tbody>
                    {w.reference_tables
                      .filter(t => t.name.toLowerCase().includes(search.toLowerCase()))
                      .map(t => (
                        <tr key={t.id}>
                          <td>
                            <strong>{t.name}</strong>
                            <div className="secondary-line">{t.periodicity}</div>
                          </td>
                          <td>{t.unit || 'A confirmar'}</td>
                          <td>{t.entries.length}</td>
                          <td>
                            {t.provenance}
                            <div className="secondary-line">
                              {t.verified ? 'Conferência declarada pelo usuário' : 'Dados não certificados'}
                            </div>
                          </td>
                          <td>
                            <Button onClick={() => setTable(structuredClone(t))}>Consultar / editar</Button>
                          </td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              </div>
              <Notice>
                Rótulos não contêm coeficientes. Informe valores e fontes manualmente; não há download ou validação implícita de
                índice oficial.
              </Notice>
            </>
          )}

          {page === 'documents' && (
            <>
              <div className="toolbar">
                <Search value={search} onChange={setSearch} placeholder="Pesquisar título ou categoria" />
                <Button primary onClick={() => setTemplate({ id: 0, name: '', category: 'Administrativo', body: '' })}>
                  <Icon name="plus" size={16} />
                  Novo modelo
                </Button>
              </div>
              <Panel
                title="Campos e cálculo de referência"
                subtitle="{{nome}}, {{cpf}}, {{nascimento}}, {{calculo}}, {{dib}}, {{escritorio}}"
              >
                <CasePicker cases={w.calculations} value={selected} onChange={setSelected} />
              </Panel>
              <div className="module-grid">
                {w.documents
                  .filter(d => [d.name, d.category].join(' ').toLowerCase().includes(search.toLowerCase()))
                  .map(d => (
                    <Panel key={d.id} title={d.name} subtitle={d.category}>
                      <p className="secondary-line">
                        {d.body.slice(0, 150)}
                        {d.body.length > 150 ? '...' : ''}
                      </p>
                      <div className="head-actions" data-ui-actions>
                        <Button onClick={() => setTemplate(structuredClone(d))}>Editar</Button>
                        <Button
                          primary
                          onClick={() => {
                            if (!selectedCase) {
                              notify('Selecione um cálculo para preencher os campos.', true);
                              return;
                            }
                            const fields: Record<string, string> = {
                              nome: selectedCase.identity.full_name ?? '[NOME NÃO INFORMADO]',
                              cpf: selectedCase.identity.cpf ?? '[CPF NÃO INFORMADO]',
                              nascimento: brDate(selectedCase.identity.birth_date),
                              calculo: selectedCase.name,
                              dib: selectedCase.dib,
                              escritorio: w.settings.office_name,
                            };
                            const text = d.body.replace(/\{\{(\w+)\}\}/g, (all, k: string) => fields[k] ?? all);
                            void api
                              .exportDocument(text)
                              .then(path => {
                                if (path) notify('Documento gerado localmente. Revise antes de utilizar.');
                              })
                              .catch(e => notify(errorMessage(e), true));
                          }}
                        >
                          Gerar texto
                        </Button>
                      </div>
                    </Panel>
                  ))}
              </div>
              {w.documents.length === 0 && (
                <Panel title="Modelos do escritório">
                  <Empty
                    title="Crie sua biblioteca local"
                    description="Modelos de texto com substituição de campos. Não incluímos petições proprietárias nem documentos de clientes como exemplos."
                  />
                </Panel>
              )}
            </>
          )}

          {page === 'reports' && (
            <ReportCenter
              workspace={w}
              selected={selected}
              onSelect={setSelected}
              onNotify={notify}
              onChangeCase={updateCaseSettings}
            />
          )}

          {page === 'agenda' && (
            <>
              <div className="toolbar">
                <Search value={search} onChange={setSearch} placeholder="Pesquisar compromisso ou data" />
                <Button
                  primary
                  onClick={() =>
                    setAppointment({
                      id: 0,
                      title: '',
                      date: localDate(),
                      time: '09:00',
                      client_id: null,
                      notes: '',
                      completed: false,
                    })
                  }
                >
                  <Icon name="plus" size={16} />
                  Novo compromisso
                </Button>
              </div>
              <div className="table-wrap">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>Compromisso</th>
                      <th>Data e hora</th>
                      <th>Cliente</th>
                      <th>Situação</th>
                      <th>Ação</th>
                    </tr>
                  </thead>
                  <tbody>
                    {w.appointments
                      .filter(a => [a.title, a.date].join(' ').toLowerCase().includes(search.toLowerCase()))
                      .map(a => (
                        <tr key={a.id}>
                          <td>
                            <strong>{a.title}</strong>
                            <div className="secondary-line">{a.notes.slice(0, 100)}</div>
                          </td>
                          <td>
                            {a.date} · {a.time}
                          </td>
                          <td>{w.clients.find(c => c.id === a.client_id)?.name ?? 'Sem vínculo'}</td>
                          <td>
                            <Badge tone={a.completed ? 'good' : 'review'}>
                              {a.completed ? 'Concluído' : 'Aberto'}
                            </Badge>
                          </td>
                          <td>
                            <Button onClick={() => setAppointment(structuredClone(a))}>Editar / concluir</Button>
                          </td>
                        </tr>
                      ))}
                  </tbody>
                </table>
              </div>
              {w.appointments.length === 0 && (
                <Panel title="Agenda local">
                  <Empty
                    title="Nenhum compromisso registrado"
                    description="Datas e anotações do escritório, sem convites, mensagens ou sincronização externa."
                  />
                </Panel>
              )}
            </>
          )}

          {page === 'settings' && (
            <>
              <Panel title="Identidade do escritório" subtitle="Cabeçalho e rodapé dos relatórios locais.">
                <div className="form-grid" data-ui-stack>
                  <Field
                    label="Nome institucional"
                    value={activeSettings.office_name}
                    onChange={office_name => setManualSettings({ ...activeSettings, office_name })}
                  />
                  <Select
                    label="Aparência"
                    value={activeSettings.light_theme ? 'Claro' : 'Escuro'}
                    onChange={s => setManualSettings({ ...activeSettings, light_theme: s === 'Claro' })}
                    items={['Escuro', 'Claro']}
                  />
                  <label className="checkline wide">
                    <input type="checkbox" checked={motionPolicy.reduced} onChange={motionPolicy.toggle} />
                    Reduzir animações
                  </label>
                  <label className="field wide">
                    <span>Logo do escritório (PNG/JPEG, opcional)</span>
                    <input
                      type="file"
                      accept="image/png,image/jpeg"
                      onChange={e => {
                        const f = e.target.files?.[0];
                        if (!f) return;
                        const r = new FileReader();
                        r.onload = () => setManualSettings({ ...activeSettings, logo_data_url: String(r.result) });
                        r.readAsDataURL(f);
                      }}
                    />
                    {activeSettings.logo_data_url ? (
                      <img
                        src={activeSettings.logo_data_url}
                        alt="Prévia da logo"
                        style={{ maxHeight: 60, marginTop: 8, display: 'block' }}
                      />
                    ) : null}
                  </label>
                  <label className="field wide">
                    <span>Rodapé e observações institucionais</span>
                    <textarea
                      className="text-area"
                      value={activeSettings.report_footer}
                      onChange={e => setManualSettings({ ...activeSettings, report_footer: e.target.value })}
                    />
                  </label>
                </div>
                <div className="form-actions" data-ui-actions>
                  <Button primary disabled={saving} onClick={() => void saveAllSettings()}>
                    Salvar configurações
                  </Button>
                </div>
              </Panel>

              <Panel
                title="Fábrica de Cálculos / TRF3"
                subtitle="Integração oficial TRF3 para RMI (17 espécies), tempo, atrasados, leitor documental e índices."
                actions={
                  <Badge tone={fabricaStatus?.configured ? 'good' : 'review'}>
                    {fabricaStatusBusy
                      ? 'Verificando...'
                      : fabricaStatus
                        ? fabricaStatus.configured
                          ? 'Conectado'
                          : 'Sem chave'
                        : activeSettings.fabrica_api_key?.trim()
                          ? 'Chave local informada'
                          : 'Não testado'}
                  </Badge>
                }
              >
                {fabricaStatusError && <Notice tone="error">{fabricaStatusError}</Notice>}
                {fabricaStatus && (
                  <div className="result-strip">
                    <div>
                      <span>Situação da conexão</span>
                      <strong>{fabricaStatus.configured ? 'Pronta para consulta' : 'Chave ausente'}</strong>
                    </div>
                    <div>
                      <span>Chave do servidor</span>
                      <strong>{fabricaStatus.serverKeyAvailable ? 'Disponível' : 'Não configurada'}</strong>
                    </div>
                    {WEB_MODE ? (
                      <div>
                        <span>Chave nesta aba</span>
                        <strong>{fabricaStatus.sessionKeyConfigured ? 'Ativa nesta aba' : 'Não informada'}</strong>
                      </div>
                    ) : (
                      <div>
                        <span>Cofre local</span>
                        <strong>{fabricaStatus.vaultConfigured ? 'Configurado' : 'Não gravado'}</strong>
                      </div>
                    )}
                    <div>
                      <span>Endereço oficial</span>
                      <strong>{fabricaStatus.upstream}</strong>
                    </div>
                  </div>
                )}
                <div className="form-grid" data-ui-stack>
                  {WEB_MODE ? (
                    <Field
                      type="password"
                      label="Chave da API da Fábrica de Cálculos (opcional na sessão)"
                      hint="Fica só nesta aba e nunca é gravada no workspace nem no banco de dados."
                      value={webFabricaKey}
                      onChange={setWebFabricaKey}
                      placeholder="Cole aqui sua chave TRF3 se o servidor não fornecer uma"
                    />
                  ) : (
                    <Field
                      type="password"
                      label="Chave da API da Fábrica de Cálculos (opcional)"
                      hint="Fica na base local cifrada e no cofre seguro. É enviada somente à Fábrica no cabeçalho de autenticação. Não envie nome, CPF ou CNIS."
                      value={activeSettings.fabrica_api_key ?? ''}
                      onChange={fabrica_api_key => setManualSettings({ ...activeSettings, fabrica_api_key })}
                    />
                  )}
                </div>
                <div className="head-actions" data-ui-actions style={{ marginTop: 14 }}>
                  <Button disabled={fabricaStatusBusy} onClick={() => void testFabricaConnection()}>
                    {fabricaStatusBusy ? 'Testando...' : 'Testar conexão TRF3'}
                  </Button>
                  <Button
                    primary
                    disabled={fabricaStatusBusy || saving}
                    onClick={() => void saveFabricaKeyFromSettings()}
                  >
                    {WEB_MODE ? 'Salvar chave na sessão' : 'Salvar chave TRF3'}
                  </Button>
                  <Button disabled={fabricaStatusBusy || saving} onClick={() => void clearFabricaKeyFromSettings()}>
                    Limpar chave TRF3
                  </Button>
                </div>
              </Panel>

              <Panel
                title="Armazenamento e backup"
                subtitle="Base cifrada pelo usuário do Windows; sem credenciais externas."
              >
                <div className="work-item">
                  <Icon name="shield" />
                  <div className="work-copy">
                    <strong>Proteção local DPAPI</strong>
                    <p>Arquivos corrompidos ou revisões conflitantes bloqueiam gravações. O aplicativo nunca grava em texto claro.</p>
                  </div>
                  <Badge tone={storageReady ? 'good' : 'error'}>
                    {storageReady ? 'Disponível' : 'Bloqueado'}
                  </Badge>
                </div>
                <div className="head-actions" data-ui-actions>
                  <Button
                    onClick={() =>
                      void api
                        .backup()
                        .then(path => {
                          if (path) notify('Backup protegido copiado para o destino escolhido.');
                        })
                        .catch(e => notify(errorMessage(e), true))
                    }
                  >
                    Exportar backup protegido
                  </Button>
                </div>
                <p className="form-note">
                  O backup depende do usuário/máquina Windows. A aplicação não altera políticas do sistema nem a instalação de
                  outro aplicativo.
                </p>
              </Panel>

              <Panel
                title="Atualizações"
                subtitle="Verifica novas versões publicadas e instala com sua confirmação."
              >
                <div className="head-actions" data-ui-actions>
                  <Button
                    onClick={() => {
                      void checkForUpdates().then(r => notify(r.message, r.kind === 'error'));
                    }}
                  >
                    Verificar atualizações
                  </Button>
                </div>
                <p className="form-note">
                  A atualização baixa da release oficial e reinicia o aplicativo. Nada é enviado do seu computador.
                </p>
              </Panel>
            </>
          )}

          {page === 'coverage' && (
            <>
              <Suspense fallback={pageFallback}><CoverageMap /></Suspense>
              <Panel
                title="Investigação com REA"
                subtitle="Ferramentas reais, evidências locais e distinção entre observação e inferência."
              >
                <div className="result-strip">
                  <div>
                    <span>REA</span>
                    <strong>6.5.0 · 207 ferramentas</strong>
                  </div>
                  <div>
                    <span>Análise nativa</span>
                    <strong>1.902 funções · 1.196 strings</strong>
                  </div>
                  <div>
                    <span>Caminho PDF</span>
                    <strong>7 funções · 6 criações observadas</strong>
                  </div>
                </div>
                <p className="form-note">
                  A análise localizou o caminho de criação dos PDFs: wrapper de arquivo, stream, configuração, saída, objetos e
                  laço de relatórios. Não recuperou as fórmulas previdenciárias nem o código completo do assistente. O recurso
                  INFOEXE de 13.587.224 bytes permanece opaco. A GUI Windows foi investigada por automação/acessibilidade; o REA
                  não possui observação nativa de UI Windows nesta versão.
                </p>
              </Panel>
              <Notice>
                A existência de uma tela não comprova equivalência funcional. O relatório de testes registra separadamente fluxo
                de interface, operação real do Rust e revisão normativa pendente.
              </Notice>
            </>
          )}
        </PageTransition>
      </main>

      <nav className="mobile-dock" hidden={!mobileLayout} aria-label="Navegação móvel" data-od-id="mobile-navigation">
        {(
          [
            { id: 'home', label: 'Início', icon: 'home' },
            { id: 'clients', label: 'Clientes', icon: 'users' },
            { id: 'cases', label: 'Cálculos', icon: 'calc' },
            { id: 'reports', label: 'Relatórios', icon: 'file' },
          ] as const
        ).map(n => (
          <button
            type="button"
            key={n.id}
            aria-current={page === n.id ? 'page' : undefined}
            onClick={() => go(n.id)}
            data-od-id={'dock-' + n.id}
          >
            {mobileLayout && page === n.id && <NavIndicator id="mobile-active-page" />}
            <Icon name={n.icon} size={19} />
            <span>{n.label}</span>
          </button>
        ))}
        <button
          type="button"
          aria-label="Todas as páginas"
          aria-haspopup="dialog"
          aria-expanded={mobileMenu}
          onClick={() => setMobileMenu(true)}
          data-od-id="dock-all-pages"
        >
          <Icon name="menu" size={19} />
          <span>Páginas</span>
        </button>
      </nav>

      {mobileMenu && (
        <Modal
          title="Todas as páginas"
          onClose={() => setMobileMenu(false)}
        >
          <nav className="mobile-menu-list" aria-label="Todas as áreas">
            {NAV.map(n => (
              <button
                type="button"
                key={n.id}
                aria-current={page === n.id ? 'page' : undefined}
                aria-label={n.full ?? n.label}
                data-od-id={'menu-' + n.id}
                onClick={() => go(n.id)}
              >
                {mobileLayout && page === n.id && <NavIndicator id="mobile-menu-active-page" />}
                <Icon name={n.icon} />
                <span>{n.label}</span>
              </button>
            ))}
          </nav>
        </Modal>
      )}

      {creationChoice && (
        <Modal
          title="Novo cálculo"
          subtitle={
            creationChoice === 'RGPS'
              ? 'Cálculos do RGPS: escolha como deseja preencher.'
              : creationChoice + ': editor de dados local.'
          }
          wide
          onClose={() => setCreationChoice(null)}
        >
          <div className="new-calculation-choices" data-ui-stack>
            <div className="new-calculation-choice recommended">
              <RobotFace large />
              <div>
                <Badge tone="good">Recomendado</Badge>
                <h3>Preenchimento guiado</h3>
                <ul className="method-features" data-ui-stack>
                  <li>Entrevista guiada, etapa por etapa.</li>
                  <li>Importação do CNIS e revisão antes do cálculo.</li>
                </ul>
                <Button primary disabled={creationChoice !== 'RGPS'} onClick={() => begin(creationChoice, true)}>
                  Usar o assistente de cálculos
                </Button>
              </div>
            </div>
            <div className="new-calculation-choice">
              <span className="manual-choice-icon">
                <Icon name="file" size={30} />
              </span>
              <div>
                <Badge>Editor direto (Prévius + TRF3)</Badge>
                <h3>Mesa de cálculo direta</h3>
                <ul className="method-features" data-ui-stack>
                  <li>Importe o CNIS e calcule em 1 clique ou edite períodos e salários direto nas abas.</li>
                  <li>Origem dos documentos e limites preservados.</li>
                </ul>
                <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                  <Button
                    primary
                    onClick={() => {
                      setDraft({ ...blankCase(), kind: creationChoice, assistant: { ...blankInterview(), mode: 'manual' } });
                      setSelected(null);
                      setCaseTab(1);
                      setWorkbenchKey(k => k + 1);
                      setCreationChoice(null);
                      go('wizard');
                    }}
                  >
                    Importar CNIS direto
                  </Button>
                  <Button onClick={() => begin(creationChoice, false)}>Não usar o assistente</Button>
                </div>
              </div>
            </div>
          </div>
          <p className="form-note">
            Cada cálculo é separado. O assistente não confirma sexo, deficiência, CTC ou datas que o documento não informa.
          </p>
        </Modal>
      )}

      {atriumImportOpen && api.atrium && (
        <AtriumImportModal
          atrium={api.atrium}
          workspace={w}
          saving={saving}
          onClose={() => setAtriumImportOpen(false)}
          onImport={async (next, message) => {
            await commit(next, true);
            setAtriumImportOpen(false);
            notify(message);
          }}
        />
      )}

      {atriumSendClient && api.atrium && (
        <AtriumSendModal
          atrium={api.atrium}
          workspace={w}
          client={atriumSendClient}
          saving={saving}
          onClose={() => setAtriumSendClient(null)}
          onSent={async (next, message) => {
            await commit(next, true);
            notify(message);
          }}
        />
      )}

      {client && (
        <Modal
          title={client.id ? 'Editar cliente' : 'Novo cliente'}
          subtitle="Dados privados da base local, com origem declarada."
          onClose={() => {
            setFormError(null);
            setClient(null);
          }}
        >
          <div className="form-grid" data-ui-stack>
            <Field
              label="Nome"
              required
              value={client.name}
              onChange={name => setClient({ ...client, name })}
              error={fieldError('name')}
            />
            <Field label="CPF / CNPJ" value={client.cpf} onChange={cpf => setClient({ ...client, cpf })} />
            <Field label="E-mail" type="email" value={client.email} onChange={email => setClient({ ...client, email })} />
            <Field label="Telefone" value={client.phone} onChange={phone => setClient({ ...client, phone })} />
            <Field
              label="Nascimento"
              value={client.birth_date}
              onChange={birth_date => setClient({ ...client, birth_date })}
              placeholder="DD/MM/AAAA"
              error={fieldError('birth')}
            />
            <Select
              label="Sexo cadastral"
              value={client.sex ?? ''}
              onChange={sex => setClient({ ...client, sex: sex || null })}
              items={['', 'Feminino', 'Masculino', 'A confirmar']}
            />
            <label className="field wide">
              <span>Observações</span>
              <textarea
                className="text-area"
                value={client.notes}
                onChange={e => setClient({ ...client, notes: e.target.value })}
              />
            </label>
          </div>
          <div className="form-actions" data-ui-actions>
            <Button
              onClick={() => {
                setFormError(null);
                setClient(null);
              }}
            >
              Cancelar
            </Button>
            <Button primary disabled={saving} onClick={() => void saveClient()}>
              Salvar cliente
            </Button>
          </div>
        </Modal>
      )}

      {appointment && (
        <Modal
          title="Compromisso local"
          subtitle="Sem convites, notificações ou comunicação a terceiros."
          onClose={() => {
            setFormError(null);
            setAppointment(null);
          }}
        >
          <div className="form-grid" data-ui-stack>
            <Field
              label="Título"
              required
              value={appointment.title}
              onChange={title => setAppointment({ ...appointment, title })}
              error={fieldError('title')}
            />
            <label className="field">
              <span>Cliente</span>
              <select
                className="input"
                value={appointment.client_id ?? ''}
                onChange={e =>
                  setAppointment({ ...appointment, client_id: e.target.value ? Number(e.target.value) : null })
                }
              >
                <option value="">Sem vínculo</option>
                {w.clients
                  .filter(c => !c.archived)
                  .map(c => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
              </select>
            </label>
            <Field
              label="Data"
              required
              value={appointment.date}
              onChange={date => setAppointment({ ...appointment, date })}
              placeholder="DD/MM/AAAA"
              error={fieldError('date')}
            />
            <Field
              label="Hora"
              required
              type="time"
              value={appointment.time}
              onChange={time => setAppointment({ ...appointment, time })}
              error={fieldError('time')}
            />
            <label className="field wide">
              <span>Observações</span>
              <textarea
                className="text-area"
                value={appointment.notes}
                onChange={e => setAppointment({ ...appointment, notes: e.target.value })}
              />
            </label>
          </div>
          <label className="checkline" style={{ marginTop: 16 }}>
            <input
              type="checkbox"
              checked={appointment.completed}
              onChange={e => setAppointment({ ...appointment, completed: e.target.checked })}
            />
            Compromisso concluído
          </label>
          <div className="form-actions" data-ui-actions>
            <Button
              onClick={() => {
                setFormError(null);
                setAppointment(null);
              }}
            >
              Cancelar
            </Button>
            <Button primary disabled={saving} onClick={() => void saveAppointment()}>
              Salvar compromisso
            </Button>
          </div>
        </Modal>
      )}

      {template && (
        <Modal
          title="Modelo de documento"
          subtitle="Texto local com campos substituíveis."
          wide
          onClose={() => {
            setFormError(null);
            setTemplate(null);
          }}
        >
          <div className="form-grid" data-ui-stack>
            <Field
              label="Título"
              required
              value={template.name}
              onChange={name => setTemplate({ ...template, name })}
              error={fieldError('name')}
            />
            <Field
              label="Categoria"
              value={template.category}
              onChange={category => setTemplate({ ...template, category })}
            />
          </div>
          <p className="form-note">
            Campos: {'{{nome}} · {{cpf}} · {{nascimento}} · {{calculo}} · {{dib}} · {{escritorio}}'}
          </p>
          <textarea
            aria-label="Conteúdo do modelo"
            aria-invalid={formError?.field === 'body' ? true : undefined}
            aria-describedby={formError?.field === 'body' ? 'template-body-error' : undefined}
            className="text-area"
            style={{ minHeight: 330 }}
            value={template.body}
            onChange={e => setTemplate({ ...template, body: e.target.value })}
          />
          {formError?.field === 'body' && (
            <span className="field-error" id="template-body-error" role="alert">
              {formError.message}
            </span>
          )}
          <div className="form-actions" data-ui-actions>
            <Button
              onClick={() => {
                setFormError(null);
                setTemplate(null);
              }}
            >
              Cancelar
            </Button>
            <Button primary disabled={saving} onClick={() => void saveTemplate()}>
              Salvar modelo
            </Button>
          </div>
        </Modal>
      )}

      {table && (
        <Modal
          title="Tabela de referência"
          subtitle="Série manual com fonte e revisão declaradas; sem valores oficiais presumidos."
          wide
          onClose={() => {
            setFormError(null);
            setTable(null);
          }}
        >
          <div className="form-grid" data-ui-stack>
            <Field
              label="Nome"
              required
              value={table.name}
              onChange={name => setTable({ ...table, name })}
              error={fieldError('name')}
            />
            <Field label="Unidade" value={table.unit} onChange={unit => setTable({ ...table, unit })} />
            <Field
              label="Periodicidade"
              value={table.periodicity}
              onChange={periodicity => setTable({ ...table, periodicity })}
            />
            <Field
              label="Fonte / versão"
              required
              value={table.provenance}
              onChange={provenance => setTable({ ...table, provenance })}
              error={fieldError('provenance')}
            />
          </div>
          <div className="table-wrap" style={{ marginTop: 18, maxHeight: 290 }}>
            <table className="data-table">
              <thead>
                <tr>
                  <th>Período</th>
                  <th>Valor nominal</th>
                  <th>Observação</th>
                </tr>
              </thead>
              <tbody>
                {table.entries.map((e, i) => (
                  <tr key={i}>
                    {(['period', 'value', 'note'] as const).map(k => (
                      <td key={k}>
                        <input
                          className="inline-input"
                          aria-label={k + ' ' + (i + 1)}
                          value={e[k]}
                          onChange={x =>
                            setTable({
                              ...table,
                              entries: table.entries.map((a, j) => (j === i ? { ...a, [k]: x.target.value } : a)),
                            })
                          }
                        />
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="head-actions" data-ui-actions style={{ marginTop: 14 }}>
            <Button onClick={() => setTable({ ...table, entries: [...table.entries, { period: '', value: '', note: '' }] })}>
              <Icon name="plus" size={15} />
              Adicionar período
            </Button>
            <label className="checkline">
              <input
                type="checkbox"
                checked={table.verified}
                onChange={e => setTable({ ...table, verified: e.target.checked })}
              />
              Conferência manual declarada
            </label>
          </div>
          <div className="form-actions" data-ui-actions>
            <Button
              onClick={() => {
                setFormError(null);
                setTable(null);
              }}
            >
              Cancelar
            </Button>
            <Button primary disabled={saving} onClick={() => void saveTable()}>
              Salvar tabela
            </Button>
          </div>
        </Modal>
      )}

      {palette && (
        <Modal
          title="Buscar no escritório"
          subtitle="Pesquisa local, sem envio de dados a modelos."
          onClose={() => setPalette(false)}
        >
          <Search value={paletteQuery} onChange={setPaletteQuery} placeholder="Cliente, cálculo ou área" />
          <div className="quick-search" style={{ marginTop: 18 }}>
            {NAV.filter(n => n.label.toLowerCase().includes(paletteQuery.toLowerCase())).map(n => (
              <button key={n.id} className="palette__item" onClick={() => go(n.id)}>
                <Icon name={n.icon} size={18} />
                <span>{n.label}</span>
                <Badge>Área</Badge>
              </button>
            ))}
            {w.clients
              .filter(c => !c.archived && c.name.toLowerCase().includes(paletteQuery.toLowerCase()))
              .slice(0, 6)
              .map(c => (
                <button
                  key={'c' + c.id}
                  className="palette__item"
                  onClick={() => {
                    setClient(structuredClone(c));
                    setPalette(false);
                  }}
                >
                  <Icon name="users" size={18} />
                  <span>{c.name}</span>
                  <Badge>Cliente</Badge>
                </button>
              ))}
            {w.calculations
              .filter(c => !c.archived && c.name.toLowerCase().includes(paletteQuery.toLowerCase()))
              .slice(0, 6)
              .map(c => (
                <button key={'k' + c.id} className="palette__item" onClick={() => openCase(c)}>
                  <Icon name="calc" size={18} />
                  <span>{c.name}</span>
                  <Badge>Cálculo</Badge>
                </button>
              ))}
          </div>
        </Modal>
      )}

      {toast?.error && <ToastMessage toast={toast} onClose={() => setToast(null)} />}
    </div>
  );
}

function ToastMessage({ toast, onClose }: { toast: { message: string; error: boolean }; onClose: () => void }) {
  return <LucianToast key={toast.message} message={toast.message} error={toast.error} onClose={onClose} />;
}

function Search({
  value,
  onChange,
  placeholder,
}: {
  value: string;
  onChange: (s: string) => void;
  placeholder: string;
}) {
  return (
    <div className="searchbox">
      <Icon name="search" size={17} />
      <input aria-label={placeholder} value={value} onChange={e => onChange(e.target.value)} placeholder={placeholder} />
    </div>
  );
}

function CasePicker({
  cases,
  value,
  onChange,
}: {
  cases: Calculation[];
  value: number | null;
  onChange: (n: number | null) => void;
}) {
  return (
    <select
      className="input"
      aria-label="Selecionar cálculo"
      value={value ?? ''}
      onChange={e => onChange(e.target.value ? Number(e.target.value) : null)}
    >
      <option value="">Selecione um cálculo</option>
      {cases
        .filter(c => !c.archived)
        .map(c => (
          <option key={c.id} value={c.id}>
            {c.name}
          </option>
        ))}
    </select>
  );
}

export default function App() {
  return (
    <MotionProvider>
      <WorkspaceApp />
    </MotionProvider>
  );
}
