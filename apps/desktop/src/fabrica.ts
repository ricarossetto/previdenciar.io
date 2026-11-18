import type {Calculation, Json} from './model';
import {isoDate, parseMoney} from './model';
import {LINK_KINDS} from './assistantReview';

export interface FabricaSalario {
  competencia: string;
  valor: string;
  excluido?: boolean;
  indicadores?: string[];
}
export interface FabricaPeriodo {
  dataInicial: string;
  dataFinal: string;
  descricao?: string;
  formaContagem?: string;
  naturezaVinculo?: string;
  contarCarencia?: boolean;
  prioritario?: boolean;
  indicadores?: string[];
  salarios: FabricaSalario[];
}
export interface FabricaInput {
  dataApuracao: string;
  dataNascimento: string;
  sexo: 'M'|'F';
  especie: number;
  isencaoCarencia: boolean;
  tipo: 'COMUM'|'PCD'|'RURICOLA';
  fatoGerador?: string;
  der?: string;
  fundamento?: string;
  marcoTemporal?: string;
  aplicarIrsmFev94?: boolean;
  calcularRmiPcdComRegraAnterior?: boolean;
  aplicarArt3Lei11718?: boolean;
  numeroDependentes?: number;
  coeficienteAuxilioAcidente?: number;
  dataAcidente?: string;
  criterioSalarioMaternidade?: string;
  descarteManual?: string[];
  demonstrativo?: boolean;
  permalink?: boolean;
  periodos: FabricaPeriodo[];
}

export const FABRICA_ESPECIES: { code: number; label: string; category: string }[] = [
  { code: 42, label: '42 — Aposentadoria por Tempo de Contribuição / Programada', category: 'aposentadoria' },
  { code: 41, label: '41 — Aposentadoria por Idade', category: 'aposentadoria' },
  { code: 46, label: '46 — Aposentadoria Especial', category: 'especial' },
  { code: 57, label: '57 — Aposentadoria de Professor', category: 'aposentadoria' },
  { code: 31, label: '31 — Auxílio por Incapacidade Temporária', category: 'incapacidade' },
  { code: 32, label: '32 — Aposentadoria por Incapacidade Permanente', category: 'incapacidade' },
  { code: 36, label: '36 — Auxílio-Acidente Previdenciário', category: 'acidentario' },
  { code: 21, label: '21 — Pensão por Morte Previdenciária', category: 'dependente' },
  { code: 25, label: '25 — Auxílio-Reclusão', category: 'dependente' },
  { code: 80, label: '80 — Salário-Maternidade', category: 'maternidade' },
  { code: 87, label: '87 — BPC / LOAS Pessoa com Deficiência', category: 'assistencial' },
  { code: 88, label: '88 — BPC / LOAS Idoso', category: 'assistencial' },
  { code: 91, label: '91 — Auxílio por Incapacidade Temporária Acidentário', category: 'acidentario' },
  { code: 92, label: '92 — Aposentadoria por Incapacidade Permanente Acidentária', category: 'acidentario' },
  { code: 93, label: '93 — Pensão por Morte Acidentária', category: 'acidentario' },
  { code: 94, label: '94 — Auxílio-Acidente Acidentário', category: 'acidentario' },
  { code: 95, label: '95 — Auxílio-Suplementar por Acidente de Trabalho', category: 'acidentario' },
];

export const FABRICA_DOC_TYPES: { id: 'cnis'|'carta'|'rdctc'|'pap'|'prevjud'; label: string }[] = [
  { id: 'cnis', label: 'CNIS (Extrato Previdenciário)' },
  { id: 'carta', label: 'Carta de Concessão' },
  { id: 'rdctc', label: 'RDCTC (Relação de Remunerações)' },
  { id: 'pap', label: 'PAP / Processo Administrativo' },
  { id: 'prevjud', label: 'PREVJUD / Dossiê Previdenciário' },
];

export const FABRICA_INDEX_TYPES: { id: string; label: string }[] = [
  { id: 'tabela', label: 'Tabela de índices mensais' },
  { id: 'reajuste', label: 'Reajuste de benefícios' },
  { id: 'moedas', label: 'Moedas históricas' },
  { id: 'tabua-mortalidade', label: 'Tábua de mortalidade IBGE' },
  { id: 'adiantamento', label: 'Adiantamentos / abonos' },
  { id: 'limites', label: 'Limites (Piso e Teto RGPS)' },
  { id: 'limite-baixa-renda-reclusao', label: 'Limite Baixa Renda (Auxílio-Reclusão)' },
  { id: 'os121', label: 'OS 121 (Ajustes)' },
  { id: 'reajuste-pre-real', label: 'Reajustes pré-Real' },
];

export interface FabricaRequisito {
  nome: string;
  rotulo?: string|null;
  exigido?: unknown;
  apurado?: unknown;
  cumprido?: boolean|null;
  porExtenso?: string|null;
  implementadoEm?: unknown;
  nota?: string|null;
  inconsistencia?: boolean|null;
}
export interface FabricaDadosRmi {
  rmi?: string|null;
  salarioBeneficio?: string|null;
  piso?: string|null;
  teto?: string|null;
  rmiReajustada?: string|null;
  coeficiente?: number|string|null;
  divisor?: number|null;
  totalCompetenciasConsideradas?: number|null;
  competenciaInicial?: string|null;
  competenciaFinal?: string|null;
  aplicouFator?: boolean|null;
  indiceReposicao?: number|string|null;
}
export interface FabricaBeneficio {
  especie?: number|null;
  descricao?: string|null;
  chaveFundamento?: string|null;
  fundamento?: string|null;
  inicioVigencia?: string|null;
  terminoVigencia?: string|null;
  alerta?: string|null;
  temDireito?: boolean|null;
  dadosRmi?: FabricaDadosRmi;
  requisitos?: FabricaRequisito[];
}
export interface FabricaProvider {
  maiorRmi: string;
  moeda?: {nome?: string; simbolo?: string; inicio?: string}|null;
  beneficios: FabricaBeneficio[];
  conclusao?: {completo?: boolean; motivoInterrupcao?: string; limitesAtingidos?: unknown}|null;
  linhasDescartes?: number;
  resultados?: number;
  demonstrativoPdf?: string|null;
  permalink?: string|null;
  messages?: string[];
}
export interface FabricaSummary {
  provider: FabricaProvider;
  source_url: string;
  retrieved_at: string;
  request_sha256: string;
  response_sha256: string;
  bytes: number;
  api: string;
  version: number;
  /** Added by the client to detect edits after the conference. */
  inputFingerprint?: string;
}

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;
const ISO_MONTH = /^\d{4}-(0[1-9]|1[0-2])$/;
const CANONICAL_MONEY = /^(0|[1-9]\d*)\.(\d{1,2})$/;

function iso(value: string|undefined|null): string|null {
  if (!value) return null;
  const text = value.trim();
  if (ISO_DATE.test(text)) return text;
  return isoDate(text);
}

function canonicalMoney(value: string|undefined|null): string|null {
  if (!value) return null;
  const text = value.trim();
  const direct = CANONICAL_MONEY.exec(text);
  if (direct) return direct[1] + '.' + direct[2].padEnd(2, '0');
  return parseMoney(text);
}

function policyOf(c: Calculation): Record<string, unknown>|null {
  const raw = c.options.rule_policy.trim();
  if (!raw.startsWith('{')) return null;
  try {
    const parsed = JSON.parse(raw) as unknown;
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
      ? parsed as Record<string, unknown>
      : null;
  } catch {
    return null;
  }
}

function inside(competence: string, start: string, end: string): boolean {
  const month = competence.slice(0, 7);
  return month >= start.slice(0, 7) && month <= end.slice(0, 7);
}

// Códigos catalogados no contrato local. Nenhum código libera resultado
// automaticamente: a semântica financeira de AVRC-DEF e dos demais não está
// confirmada neste fluxo. Indicador é alerta, não prova de invalidez; a
// pendência documental precisa ser resolvida antes do envio. Código fora da
// lista também falha fechado.
const KNOWN_INDICATORS = new Set([
  'PRPPS',
  'PREC-MENOR-MIN',
  'IREM-INDPEND',
  'IREC-INDPEND',
  'PREM-EMPR',
  'PREM-FVIN',
  'PEXT',
  'AVRC-DEF',
]);

function indicatorCodes(value: string): string[] {
  return value.split(/[\s,;]+/).map(code => code.trim().toUpperCase()).filter(Boolean);
}

function indicatorIssue(value: string): string | null {
  const codes = indicatorCodes(value);
  if (!codes.length) return null;
  const unknown = codes.filter(code => !KNOWN_INDICATORS.has(code));
  if (unknown.length) {
    return `Indicador(es) ${unknown.join(', ')} não catalogado(s); revise a origem e registre a resolução documental antes do envio.`;
  }
  return `Indicador(es) ${codes.join(', ')} pendente(s) de revisão documental; código catalogado não tem semântica financeira confirmada neste fluxo e não libera resultado. Resolva a pendência documental antes do envio.`;
}

/**
 * The provider's `tipo` is global (`dadosSegurado.tipo`), not per link. A rural
 * declaration is sent only when every source-reviewed link proves rural
 * affiliation and none mixes another category. Manual-only links are covered
 * by the explicit global declaration.
 */
function ruralScopeConfirmed(c: Calculation): boolean {
  const sources = new Map((c.cnis?.links ?? []).map(link => [link.sequence, link]));
  for (const link of c.manual_links.filter(item => !item.source_review?.excluded)) {
    const sequence = link.source_review?.sequence;
    if (sequence === undefined) continue;
    const source = sources.get(sequence);
    if (!source || source.prpps) return false;
    const affiliation = (source.affiliation ?? '').trim().toLowerCase();
    if (!affiliation.includes('rural')) return false;
    if (['dom','individual','avulso','facultativo','servidor','rpps','especial'].some(term => affiliation.includes(term))) return false;
  }
  return true;
}
/**
 * Builds the Fábrica de Cálculos payload from the reviewed case. It never reads
 * name, CPF, NIT, CNIS or notes. Any gap returns issues and no partial payload.
 */
export function buildFabricaInput(c: Calculation): { input: FabricaInput|null; issues: string[] } {
  const issues: string[] = [];
  const push = (message: string) => { if (!issues.includes(message)) issues.push(message); };
  const especie = Number(c.especie ?? '42');
  if (![31, 32, 41, 42].includes(especie)) push('A conferência online atende 31, 32, 41 e 42.');
  const nascimento = iso(c.identity.birth_date);
  if (!nascimento) push('Informe a data de nascimento em formato válido.');
  const dib = iso(c.dib);
  if (!dib) push('Informe a DIB em formato válido.');
  const sexo: 'M'|'F'|null = c.sex === 'Masculino' ? 'M' : c.sex === 'Feminino' ? 'F' : null;
  if (!sexo) push('Confirme o sexo cadastral antes da conferência.');

  const policy = policyOf(c);
  if (['Leve', 'Moderada', 'Grave'].includes(c.disability ?? '') || Boolean(policy?.lc142)) {
    push('Cenário PcD: use o módulo LC 142/2013; a conferência online cobre as espécies comuns.');
  }

  const links = c.manual_links.filter(link => !link.source_review?.excluded);
  const sources = new Map((c.cnis?.links ?? []).map(source => [source.sequence, source]));
  if (!links.length) push('Confirme ao menos um vínculo antes da conferência.');
  links.forEach(link => {
    if (!link.confirmed) push('Confirme todos os vínculos usados na conferência.');
    const regime = link.regime.trim().toUpperCase();
    if (regime !== 'RGPS') {
      push(`Vínculo com regime ${link.regime.trim() || 'ausente'} não é RGPS; confirme o regime com documento próprio antes do envio.`);
    }
    const linkIndicators = indicatorIssue(link.indicators);
    if (linkIndicators) push(linkIndicators);
    const sequence = link.source_review?.sequence;
    if (sequence !== undefined) {
      const source = sources.get(sequence);
      if (!source) push('Vínculo aponta origem ausente; revise a proveniência antes do envio.');
      else {
        if (source.prpps || source.indicators.some(code => code.trim().toUpperCase() === 'PRPPS')) {
          push('Origem PRPPS/CTC permanece pendente mesmo com vínculo confirmado; não enviar sem prova documental específica.');
        }
        const affiliation = (source.affiliation ?? '').trim().toLowerCase();
        if (affiliation.includes('servidor') || affiliation.includes('rpps')) {
          push('Afiliação de origem indica RPPS; confirme regime e CTC antes do envio.');
        }
      }
    }
    const start = iso(link.start_date);
    const end = iso(link.end_date);
    if (!start || !end) push('Todo vínculo precisa de início e fim confirmados.');
    else if (end < start) push('Há vínculo com fim anterior ao início.');
    else if (dib && end > dib) push('Há vínculo posterior à DIB; a conferência não projeta tempo futuro.');
  });

  const periods = links.map(link => {
    const start = iso(link.start_date);
    const end = iso(link.end_date);
    return start && end && end >= start ? { start, end, salarios: [] as FabricaSalario[] } : null;
  });

  if (!c.manual_salaries.length) push('Confirme ao menos uma remuneração antes da conferência.');
  const seen = new Set<string>();
  const orphans: string[] = [];
  for (const salary of c.manual_salaries) {
    if (!salary.confirmed) push('Confirme todas as remunerações usadas na conferência.');
    const salaryIndicators = indicatorIssue(salary.indicators);
    if (salaryIndicators) push(salaryIndicators);
    const competence = salary.competence.trim();
    if (!ISO_MONTH.test(competence)) { push('Há competência de remuneração inválida.'); continue; }
    if (seen.has(competence)) push('Há competência repetida entre as remunerações.');
    seen.add(competence);
    const value = canonicalMoney(salary.amount);
    if (!value) { push('Há remuneração com valor inválido.'); continue; }
    const index = periods.findIndex(period => period !== null && inside(competence, period.start, period.end));
    if (index < 0) { orphans.push(competence); continue; }
    periods[index]!.salarios.push({ competencia: competence, valor: value });
  }
  if (orphans.length) push('Há remuneração fora dos vínculos confirmados; corrija antes da conferência.');
  for (let first = 0; first < periods.length; first += 1) {
    for (let second = first + 1; second < periods.length; second += 1) {
      const a = periods[first], b = periods[second];
      if (a && b && a.start <= b.end && b.start <= a.end) {
        push('Há períodos sobrepostos; resolva a concomitância antes da conferência online.');
      }
    }
  }
  const kind = typeof policy?.manual_link_kind === 'string' ? policy.manual_link_kind : '';
  // COMUM recebe só os cinco tipos RGPS do adaptador local. RURICOLA exige a
  // declaração global `rural` e prova rural em cada vínculo de origem; especial,
  // servidor e legado continuam pendentes.
  const tipo: 'COMUM'|'RURICOLA'|null = kind === 'rural'
    ? (ruralScopeConfirmed(c) ? 'RURICOLA' : null)
    : LINK_KINDS.includes(kind) ? 'COMUM' : null;
  if (!tipo) push(kind === 'rural'
    ? 'Declaração rural global sem prova de que todos os vínculos de origem são rurais; separe o caso ou revise a categoria.'
    : 'Categoria manual ausente ou não suportada; escolha uma categoria reconhecida antes da conferência online.');
  if (issues.length || !nascimento || !dib || !sexo || !tipo) return { input: null, issues };
  const native = policy?.native_options && typeof policy.native_options === 'object'
    ? policy.native_options as Record<string, unknown>
    : null;
  const origin = typeof native?.incapacidadeOrigem === 'string' ? native.incapacidadeOrigem : '';
  const isencaoCarencia = [31, 32].includes(especie)
    && ['acidente', 'doenca_ocupacional', 'doenca_do_trabalho', 'doenca_listada'].includes(origin);

  return {
    input: {
      dataApuracao: dib,
      dataNascimento: nascimento,
      sexo,
      especie,
      isencaoCarencia,
      tipo,
      ...([31, 32].includes(especie) ? { fatoGerador: dib } : {}),
      periodos: periods.filter((period): period is { start: string; end: string; salarios: FabricaSalario[] } => period !== null)
        .map(period => ({ dataInicial: period.start, dataFinal: period.end, salarios: period.salarios })),
    },
    issues: [],
  };
}

/** Local RMI used for the side-by-side comparison. It never invents a value. */
export function localRmi(result: Json): { rmi: string|null; corrected: boolean } {
  if (!result || typeof result !== 'object') return { rmi: null, corrected: false };
  const source = result as Record<string, Json>;
  const envelope = (source.data ?? source.result ?? {}) as Record<string, Json>;
  const data = (envelope.native_result ?? envelope) as Record<string, Json>;
  const summary = (data.resumo ?? {}) as Record<string, Json>;
  const transitions = (data.transicoes ?? {}) as Record<string, Json>;
  const raw = transitions.rmi_escolhida ?? summary.rmi;
  const params = (data.parametros ?? {}) as Record<string, Json>;
  return {
    rmi: raw === null || raw === undefined ? null : canonicalMoney(typeof raw === 'string' ? raw : String(raw)),
    corrected: Boolean(params.correcao_ativa ?? data.correcao),
  };
}

export function cents(value: string|null|undefined): bigint|null {
  const canonical = canonicalMoney(value ?? null);
  return canonical ? BigInt(canonical.replace('.', '')) : null;
}

export function difference(
  local: string|null|undefined,
  provider: string|null|undefined,
): { cents: bigint; percent: string }|null {
  const a = cents(local), b = cents(provider);
  if (a === null || b === null || a === 0n) return null;
  const delta = b - a;
  const percent = ((Number(delta) / Number(a)) * 100).toFixed(1);
  return { cents: delta, percent };
}

function clampIsoDate(value: string, minDate: string, maxDate: string): string {
  if (value < minDate) return minDate;
  if (value > maxDate) return maxDate;
  return value;
}

function endOfMonthIso(comp: string): string {
  const [y, m] = comp.split('-').map(Number);
  if (!y || !m) return comp + '-28';
  return new Date(Date.UTC(y, m, 0)).toISOString().slice(0, 10);
}

/**
 * Direct TRF3 payload builder: works immediately from CNIS extract or manual/reviewed
 * entries without requiring prior local engine approval, supports all 17 species,
 * COMUM/PCD/RURICOLA, and overlapping concomitant links natively handled by TRF3.
 */
export function buildFabricaInputDirect(
  c: Calculation,
  overrides?: {
    especie?: number;
    tipo?: 'COMUM'|'PCD'|'RURICOLA';
    demonstrativo?: boolean;
    permalink?: boolean;
    fundamento?: string;
    der?: string;
    isencaoCarencia?: boolean;
  },
): { input: FabricaInput|null; issues: string[]; sourceMode: 'reviewed'|'cnis_direct'|'manual_direct' } {
  const reviewed = buildFabricaInput(c);
  const overrideRequestsCustom = Boolean(
    overrides?.especie !== undefined
    || overrides?.tipo !== undefined
    || overrides?.demonstrativo
    || overrides?.permalink
    || overrides?.fundamento
    || overrides?.der
    || overrides?.isencaoCarencia !== undefined,
  );
  if (reviewed.input && !overrideRequestsCustom) {
    return { input: reviewed.input, issues: [], sourceMode: 'reviewed' };
  }

  const issues: string[] = [];
  const push = (m: string) => { if (!issues.includes(m)) issues.push(m); };
  const MIN_TRF3_DATE = '1973-06-11';
  const validSpecies = new Set(FABRICA_ESPECIES.map(e => e.code));
  const especie = overrides?.especie ?? Number(c.especie || '42');
  if (!validSpecies.has(especie)) push(`Espécie ${especie} não suportada pela Fábrica TRF3.`);

  const dib = iso(c.dib);
  if (!dib) push('Informe a DIB em formato válido (DD/MM/AAAA ou AAAA-MM-DD).');

  const birthRaw = c.identity.birth_date ?? c.cnis?.identity?.birth_date ?? null;
  const nascimento = iso(birthRaw);
  if (!nascimento) push('Informe a data de nascimento do segurado.');

  const cnisSex = (c.cnis?.identity as { sex?: string|null }|undefined)?.sex?.toLowerCase();
  const rawSex = (c.sex ?? '').trim().toLowerCase() || cnisSex || '';
  const sexo: 'M'|'F'|null =
    rawSex.startsWith('m') ? 'M'
    : rawSex.startsWith('f') ? 'F'
    : null;
  if (!sexo) push('Informe o sexo do segurado (Masculino ou Feminino).');

  const policy = policyOf(c);
  const kind = typeof policy?.manual_link_kind === 'string' ? policy.manual_link_kind : '';
  const isPcd = ['Leve', 'Moderada', 'Grave'].includes(c.disability ?? '');
  const tipo: 'COMUM'|'PCD'|'RURICOLA' = overrides?.tipo
    ?? (isPcd ? 'PCD' : kind === 'rural' ? 'RURICOLA' : 'COMUM');

  const isCnisRpps = (l: NonNullable<Calculation['cnis']>['links'][number]): boolean => {
    if (l.prpps) return true;
    const aff = (l.affiliation ?? '').toLowerCase();
    if (aff.includes('rpps') || aff.includes('prpps') || aff.includes('servidor')) return true;
    return l.indicators.some(ind => {
      const upper = ind.toUpperCase();
      return upper.includes('PRPPS') || upper.includes('RPPS');
    });
  };
  const cnisBySeq = new Map((c.cnis?.links ?? []).map(l => [l.sequence, l]));
  const isManualRpps = (l: Calculation['manual_links'][number]): boolean => {
    const regime = (l.regime ?? '').trim().toUpperCase();
    if (regime === 'RPPS' || regime === 'PRPPS' || regime.includes('RPPS')) return true;
    const ind = (l.indicators ?? '').toUpperCase();
    if (ind.includes('PRPPS') || ind.includes('RPPS')) return true;
    const seq = l.source_review?.sequence;
    if (typeof seq === 'number') {
      const src = cnisBySeq.get(seq);
      if (src && isCnisRpps(src)) return true;
    }
    return false;
  };
  const sourceLinks = (c.cnis?.links ?? []).filter(l => !isCnisRpps(l) && l.start_date);
  const activeManualLinks = c.manual_links.filter(l => !l.source_review?.excluded && !isManualRpps(l));
  const hasReviewedManual = activeManualLinks.length > 0;
  let sourceMode: 'reviewed'|'cnis_direct'|'manual_direct' = 'manual_direct';
  let periodos: FabricaPeriodo[] = [];

  if (dib && hasReviewedManual) {
    sourceMode = reviewed.input ? 'reviewed' : 'manual_direct';
    const rawPeriods: Array<{
      dataInicial: string;
      dataFinal: string;
      sourceSequence: number | null;
      centavosByComp: Map<string, number>;
    }> = [];
    for (const link of activeManualLinks) {
      const s = iso(link.start_date);
      const e = iso(link.end_date) ?? dib;
      if (!s || s > dib) continue;
      const start = clampIsoDate(s, MIN_TRF3_DATE, dib);
      const end = clampIsoDate(e, start, dib);
      rawPeriods.push({
        dataInicial: start,
        dataFinal: end,
        sourceSequence: link.source_review?.sequence ?? null,
        centavosByComp: new Map<string, number>(),
      });
    }
    const toCentavos = (canonical: string): number => {
      if (!/^\d+\.\d{2}$/.test(canonical)) return -1;
      const [whole, frac = '00'] = canonical.split('.');
      return Number(whole) * 100 + Number(frac.padEnd(2, '0').slice(0, 2));
    };
    const blockedSequences = new Set<number>(
      c.manual_links
        .filter(l => l.source_review?.excluded || isManualRpps(l))
        .map(l => l.source_review?.sequence)
        .filter((seq): seq is number => typeof seq === 'number'),
    );
    for (const src of c.cnis?.links ?? []) {
      if (isCnisRpps(src)) blockedSequences.add(src.sequence);
    }

    if (c.contribution_review?.applied) {
      for (const r of c.contribution_review.records) {
        if (r.excluded) continue;
        if (r.amount_kind !== undefined && r.amount_kind !== 'remuneration') continue;
        if (r.link_sequence !== null && blockedSequences.has(r.link_sequence)) continue;
        const comp = r.competence.trim();
        const val = canonicalMoney(r.amount);
        if (!ISO_MONTH.test(comp) || !val) continue;
        const cents = toCentavos(val);
        if (!Number.isSafeInteger(cents) || cents <= 0) continue;
        const target =
          (r.link_sequence !== null
            ? rawPeriods.find(p => p.sourceSequence === r.link_sequence && inside(comp, p.dataInicial, p.dataFinal))
            : undefined)
          ?? rawPeriods.find(p => inside(comp, p.dataInicial, p.dataFinal));
        if (target) {
          target.centavosByComp.set(comp, (target.centavosByComp.get(comp) ?? 0) + cents);
        }
      }
    } else {
      for (const s of c.manual_salaries) {
        const comp = s.competence.trim();
        const val = canonicalMoney(s.amount);
        if (!ISO_MONTH.test(comp) || !val) continue;
        const cents = toCentavos(val);
        if (!Number.isSafeInteger(cents) || cents <= 0) continue;
        const target = rawPeriods.find(p => inside(comp, p.dataInicial, p.dataFinal));
        if (target) {
          target.centavosByComp.set(comp, (target.centavosByComp.get(comp) ?? 0) + cents);
        }
      }
    }

    periodos = rawPeriods.map(p => ({
      dataInicial: p.dataInicial,
      dataFinal: p.dataFinal,
      salarios: Array.from(p.centavosByComp.entries())
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([competencia, cent]) => ({ competencia, valor: (cent / 100).toFixed(2) })),
    }));
  } else if (dib && sourceLinks.length > 0) {
    sourceMode = 'cnis_direct';
    const entries = c.cnis?.salary_entries ?? [];
    for (const link of sourceLinks) {
      const s = iso(link.start_date);
      if (!s || s > dib) continue;
      const explicitEnd = iso(link.end_date);
      const inferredEnd = iso(link.inferred_last_salary_end?.date_candidate);
      const linkWages = entries.filter(e =>
        e.link_sequence === link.sequence
        && !e.excluded_from_validated_domain
        && (e.amount_kind === undefined || e.amount_kind === 'remuneration')
        && e.amount_centavos > 0
      );
      const lastComp = linkWages.map(e => e.competence).sort().at(-1)
        ?? (link.reported_last_salary_month && ISO_MONTH.test(link.reported_last_salary_month) ? link.reported_last_salary_month : undefined);
      const fallbackEnd = explicitEnd ?? inferredEnd ?? (lastComp ? endOfMonthIso(lastComp) : dib);
      const start = clampIsoDate(s, MIN_TRF3_DATE, dib);
      const end = clampIsoDate(fallbackEnd, start, dib);
      const dedupByMonth = new Map<string, number>();
      for (const w of linkWages) {
        if (!inside(w.competence, start, end)) continue;
        dedupByMonth.set(w.competence, (dedupByMonth.get(w.competence) ?? 0) + w.amount_centavos);
      }
      const salarios: FabricaSalario[] = Array.from(dedupByMonth.entries())
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([competencia, cent]) => ({ competencia, valor: (cent / 100).toFixed(2) }));
      periodos.push({
        dataInicial: start,
        dataFinal: end,
        descricao: link.affiliation?.trim() || undefined,
        salarios,
      });
    }
  }

  if (periodos.length === 0) {
    push('Importe um CNIS ou adicione ao menos um período contributivo válido até a DIB.');
  }

  if (issues.length || !dib || !nascimento || !sexo) {
    return { input: null, issues, sourceMode };
  }

  const native = policy?.native_options && typeof policy.native_options === 'object'
    ? policy.native_options as Record<string, unknown>
    : null;
  const origin = typeof native?.incapacidadeOrigem === 'string' ? native.incapacidadeOrigem : '';
  const isencaoCarencia = overrides?.isencaoCarencia ?? (
    [31, 32, 91, 92, 93, 94, 95].includes(especie)
    && ['acidente', 'doenca_ocupacional', 'doenca_do_trabalho', 'doenca_listada'].includes(origin)
  );
  const needsFatoGerador = [21, 25, 31, 32, 36, 80, 91, 92, 93, 94, 95].includes(especie);
  const derIso = overrides?.der ? iso(overrides.der) : null;

  return {
    input: {
      dataApuracao: dib,
      dataNascimento: nascimento,
      sexo,
      especie,
      isencaoCarencia,
      tipo,
      ...(needsFatoGerador ? { fatoGerador: dib } : {}),
      ...(derIso ? { der: derIso } : {}),
      ...(overrides?.fundamento?.trim() ? { fundamento: overrides.fundamento.trim() } : {}),
      ...(overrides?.demonstrativo ? { demonstrativo: true } : {}),
      ...(overrides?.permalink ? { permalink: true } : {}),
      periodos: periodos.slice(0, 200),
    },
    issues: [],
    sourceMode,
  };
}

