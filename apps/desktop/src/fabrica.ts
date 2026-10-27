import type {Calculation, Json} from './model';
import {isoDate, parseMoney} from './model';

export interface FabricaSalario { competencia: string; valor: string }
export interface FabricaPeriodo { dataInicial: string; dataFinal: string; salarios: FabricaSalario[] }
export interface FabricaInput {
  dataApuracao: string;
  dataNascimento: string;
  sexo: 'M'|'F';
  especie: number;
  isencaoCarencia: boolean;
  tipo: 'COMUM'|'RURICOLA';
  fatoGerador?: string;
  periodos: FabricaPeriodo[];
}

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
  if (!links.length) push('Confirme ao menos um vínculo antes da conferência.');
  links.forEach(link => {
    if (!link.confirmed) push('Confirme todos os vínculos usados na conferência.');
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
  if (issues.length || !nascimento || !dib || !sexo) return { input: null, issues };

  const kind = typeof policy?.manual_link_kind === 'string' ? policy.manual_link_kind : '';
  const tipo: 'COMUM'|'RURICOLA' = kind === 'rural' || kind === 'especial' ? 'RURICOLA' : 'COMUM';
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

