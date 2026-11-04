import type { Calculation, Cnis, Json } from './model';
import { hasFinancialResult, isoDate, missingParameterKeys, PARAMETER_KEYS } from './model';

/// As oito etapas canônicas do editor. O assistente guiado usa os mesmos nomes
/// para não criar dois vocabulários sobre a mesma hipótese.
export const JOURNEY_TITLES = ['Identificação', 'CNIS', 'Cenário', 'Vínculos', 'Salários', 'Parâmetros', 'Revisão', 'Resultado'] as const;

export type JourneyAction = 'identificacao' | 'origem' | 'vinculos' | 'salarios' | 'parametros' | 'revisao' | 'execucao';

const STEP_ACTION: JourneyAction[] = ['identificacao', 'origem', 'identificacao', 'vinculos', 'salarios', 'parametros', 'revisao', 'execucao'];

export function clampJourneyStep(step: number): number {
  return Number.isFinite(step) ? Math.min(Math.max(Math.trunc(step), 0), JOURNEY_TITLES.length - 1) : 0;
}

export interface JourneyStage { index: number; title: string; action: JourneyAction }
export function journeyStage(step: number): JourneyStage {
  const index = clampJourneyStep(step);
  return { index, title: JOURNEY_TITLES[index], action: STEP_ACTION[index] };
}

export interface JourneyPrimaryAction { label: string; hint: string; targetStep: number; kind: 'navegar' | 'importar' | 'executar' }
export function primaryJourneyAction(draft: Calculation, step: number): JourneyPrimaryAction {
  switch (clampJourneyStep(step)) {
    case 0: return { label: 'Avançar para o CNIS', hint: 'Confira a identificação mínima e siga para a origem documental.', targetStep: 1, kind: 'navegar' };
    case 1: return draft.cnis
      ? { label: 'Conferir os vínculos', hint: 'Origem aplicada. Agora confira regime, datas e CTC antes de calcular.', targetStep: 3, kind: 'navegar' }
      : { label: 'Importar o CNIS', hint: 'A leitura é local; nada é salvo na base sem a sua confirmação.', targetStep: 1, kind: 'importar' };
    case 2: return { label: 'Conferir os vínculos', hint: 'Sem seguir para os vínculos, o motor não tem períodos para contar.', targetStep: 3, kind: 'navegar' };
    case 3: return { label: 'Conferir as remunerações', hint: 'Vínculo sem conferência continua como pendência, não como dado válido.', targetStep: 4, kind: 'navegar' };
    case 4: return { label: 'Abrir as opções da RMI', hint: 'Correção, descarte e limites são declarados, nunca presumidos.', targetStep: 5, kind: 'navegar' };
    case 5: return { label: 'Revisar a hipótese', hint: 'A revisão registra que os dados foram conferidos antes da execução.', targetStep: 6, kind: 'navegar' };
    case 6: return { label: 'Avançar para o resultado', hint: 'A execução envia a hipótese revisada ao motor local.', targetStep: 7, kind: 'navegar' };
    default: return { label: 'Executar a simulação', hint: 'Executa o motor local e abre a memória; nada é enviado a terceiros.', targetStep: 7, kind: 'executar' };
  }
}

export type JourneyLayerState = 'vazio' | 'parcial' | 'pronto';
export interface JourneyMetric { label: string; value: string }
export interface JourneyLayer { id: 'importado' | 'confirmado' | 'calculado'; title: string; state: JourneyLayerState; stateLabel: string; summary: string; metrics: JourneyMetric[] }

const LAYER_STATE_LABEL: Record<JourneyLayerState, string> = { vazio: 'Ainda vazio', parcial: 'Em andamento', pronto: 'Completo' };

function resultRecord(value: Json): Record<string, Json> | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, Json> : null;
}

function importedLayer(cnis: Cnis | null): JourneyLayer {
  if (!cnis) return {
    id: 'importado', title: 'Importado do CNIS', state: 'vazio', stateLabel: LAYER_STATE_LABEL.vazio,
    summary: 'Nenhuma origem CNIS aplicada. O editor manual continua disponível, com os mesmos limites de conferência.',
    metrics: [{ label: 'Vínculos lidos', value: '0' }, { label: 'Competências', value: '0' }, { label: 'Lançamentos', value: '0' }, { label: 'Avisos da leitura', value: '0' }],
  };
  return {
    id: 'importado', title: 'Importado do CNIS', state: 'pronto', stateLabel: 'Origem preservada',
    summary: 'Leitura local do documento; o PDF original não é alterado nem enviado a serviços externos.',
    metrics: [{ label: 'Vínculos lidos', value: String(cnis.links.length) }, { label: 'Competências', value: String(Object.keys(cnis.salary_months).length) }, { label: 'Lançamentos', value: String(cnis.salary_entries.length) }, { label: 'Avisos da leitura', value: String(cnis.warnings.length) }],
  };
}

function confirmedLayer(draft: Calculation): JourneyLayer {
  const links = draft.manual_links, effectiveLinks = links.filter(link => !link.source_review?.excluded), confirmedLinks = effectiveLinks.filter(link => link.confirmed).length;
  const wages = draft.manual_salaries, confirmedWages = wages.filter(wage => wage.confirmed).length;
  const missing = missingParameterKeys(draft), parametersComplete = PARAMETER_KEYS.length - missing.length;
  const reviewRequired = draft.assistant?.mode === 'assisted';
  const reviewOk = reviewRequired ? Boolean(draft.assistant?.review_confirmed) : true;
  const hasData = effectiveLinks.length > 0 || wages.length > 0;
  const ready = hasData && missing.length === 0 && confirmedLinks === effectiveLinks.length && confirmedWages === wages.length && reviewOk;
  const state: JourneyLayerState = ready ? 'pronto' : hasData || missing.length === 0 ? 'parcial' : 'vazio';
  const summary = state === 'pronto'
    ? 'Dados conferidos por declaração explícita. A revisão profissional continua necessária.'
    : hasData ? 'Parte dos dados ainda não foi conferida; o motor trata o que falta como pendência.'
    : 'Os dados de identificação e conferência ainda não foram preenchidos.';
  return {
    id: 'confirmado', title: 'Confirmado por você', state, stateLabel: LAYER_STATE_LABEL[state], summary,
    metrics: [
      { label: 'Parâmetros completos', value: parametersComplete + '/' + PARAMETER_KEYS.length },
      { label: 'Vínculos conferidos', value: confirmedLinks + '/' + effectiveLinks.length },
      { label: 'Remunerações conferidas', value: confirmedWages + '/' + wages.length },
      { label: 'Revisão registrada', value: reviewRequired ? (reviewOk ? 'Sim' : 'Não') : 'Não se aplica' },
    ],
  };
}

function calculatedLayer(draft: Calculation): JourneyLayer {
  const result = resultRecord(draft.result), blocked = result?.status === 'blocked', financial = hasFinancialResult(draft);
  const blockersCount = Array.isArray(result?.blockers) ? (result.blockers as Json[]).length : 0;
  const warningsCount = Array.isArray(result?.warnings) ? (result.warnings as Json[]).length : 0;
  if (!result) return {
    id: 'calculado', title: 'Calculado pelo motor', state: 'vazio', stateLabel: LAYER_STATE_LABEL.vazio,
    summary: 'Ainda não executado. Nenhum valor financeiro foi produzido até aqui.',
    metrics: [{ label: 'Estado', value: 'Não executado' }, { label: 'Pendências do motor', value: '—' }, { label: 'Avisos', value: '—' }, { label: 'Valor financeiro', value: 'Ausente' }],
  };
  const state: JourneyLayerState = blocked ? 'parcial' : 'pronto';
  const summary = blocked
    ? 'O motor recusou a execução e não gerou valor. As pendências continuam válidas.'
    : financial ? 'Resultado experimental do motor local; leia a memória e os avisos antes de usar.'
    : 'Execução concluída sem valor financeiro utilizável.';
  return {
    id: 'calculado', title: 'Calculado pelo motor', state, stateLabel: blocked ? 'Bloqueado' : (financial ? 'Resultado experimental' : 'Sem valor'), summary,
    metrics: [{ label: 'Estado', value: blocked ? 'Bloqueado' : String(result.status ?? 'sem status') }, { label: 'Pendências do motor', value: String(blockersCount) }, { label: 'Avisos', value: String(warningsCount) }, { label: 'Valor financeiro', value: financial ? 'Presente' : 'Ausente' }],
  };
}

export function journeyLayers(draft: Calculation): JourneyLayer[] {
  return [importedLayer(draft.cnis), confirmedLayer(draft), calculatedLayer(draft)];
}

export interface JourneyPendencyGroup { action: JourneyAction; label: string; hint: string; step: number; items: string[] }

const PENDENCY_STEP: Record<JourneyAction, number> = { identificacao: 0, origem: 1, vinculos: 3, salarios: 4, parametros: 5, revisao: 6, execucao: 7 };

function hasPcd(draft: Calculation): boolean {
  return ['Leve', 'Moderada', 'Grave'].includes(draft.disability ?? '');
}

function policyRecord(draft: Calculation): Record<string, Json> | null {
  try { return resultRecord(JSON.parse(draft.options.rule_policy || '')); } catch { return null; }
}

export function journeyPendencies(draft: Calculation): JourneyPendencyGroup[] {
  const groups: JourneyPendencyGroup[] = [];
  const cnis = draft.cnis;
  const push = (action: JourneyAction, label: string, hint: string, items: string[]) => { if (items.length) groups.push({ action, label, hint, step: PENDENCY_STEP[action], items }); };
  const identification: string[] = [];
  if (!draft.identity.full_name?.trim()) identification.push('Nome do segurado não informado');
  if (!draft.identity.birth_date || !isoDate(draft.identity.birth_date)) identification.push('Nascimento ausente ou inválido');
  if (!['Feminino', 'Masculino'].includes(draft.sex ?? '')) identification.push('Sexo cadastral sem confirmação');
  if (!draft.disability || draft.disability === 'A apurar') identification.push('Condição de deficiência sem confirmação');
  if (!isoDate(draft.dib)) identification.push('DIB ausente ou inválida');
  push('identificacao', 'Completar identificação', 'Dados cadastrais declarados pelo operador; nada é inferido pelo nome.', identification);
  push('origem', 'Aplicar a origem CNIS', cnis ? 'Origem aplicada; confira se a leitura cobre o período que importa.' : 'A leitura local do CNIS é opcional, mas é o caminho com menos digitação.', cnis ? [] : ['Nenhum CNIS aplicado ao rascunho']);
  const links = draft.manual_links, vinculos: string[] = [];
  if (cnis && links.length === 0) vinculos.push('Preparar os vínculos lidos para revisão');
  const unconfirmedLinks = links.filter(link => !link.source_review?.excluded && !link.confirmed).length;
  if (unconfirmedLinks > 0) vinculos.push(unconfirmedLinks + (unconfirmedLinks === 1 ? ' vínculo sem conferência declarada' : ' vínculos sem conferência declarada'));
  const prpps = cnis?.links.filter(link => link.prpps).length ?? 0;
  if (prpps > 0) vinculos.push(prpps + (prpps === 1 ? ' vínculo PRPPS exige CTC' : ' vínculos PRPPS exigem CTC'));
  const missingEnd = cnis?.links.filter(link => !link.end_date).length ?? 0;
  if (missingEnd > 0) vinculos.push(missingEnd + (missingEnd === 1 ? ' data final ausente na origem' : ' datas finais ausentes na origem'));
  push('vinculos', 'Revisar vínculos', 'Regime, datas e CTC são declarações conferidas, não deduções da leitura.', vinculos);
  const salarios: string[] = [];
  if (cnis && draft.manual_salaries.length === 0) salarios.push('Preparar o PBC lido para revisão');
  const unconfirmedWages = draft.manual_salaries.filter(wage => !wage.confirmed).length;
  if (unconfirmedWages > 0) salarios.push(unconfirmedWages + (unconfirmedWages === 1 ? ' remuneração sem conferência declarada' : ' remunerações sem conferência declarada'));
  if (draft.contribution_review && !draft.contribution_review.applied) salarios.push('Livro de contribuições preparado, ainda não aplicado');
  push('salarios', 'Revisar remunerações', 'Competências, valores e moeda permanecem como na origem até a conferência.', salarios);
  const parametros: string[] = [];
  const policy = policyRecord(draft);
  const lc142 = policy ? resultRecord(policy.lc142 ?? null) : null;
  if (hasPcd(draft) && lc142?.assumptions_confirmed !== true) parametros.push('Premissas do cenário PcD não confirmadas');
  const native = policy && typeof policy.native_options === 'object' && policy.native_options !== null && !Array.isArray(policy.native_options) ? policy.native_options as Record<string, Json> : null;
  const correctionReference = native && typeof native.referenciaCorrecao === 'string' ? native.referenciaCorrecao.trim() : '';
  const correctionExplicitlyOff = native?.corrigirSalarios === false;
  const correctionDeclared = Boolean(draft.options.correction_index.trim()) || correctionReference !== '' || correctionExplicitlyOff;
  if (cnis && !correctionDeclared) parametros.push('Referência de correção das séries não declarada');
  push('parametros', 'Revisar opções da RMI', 'Correção, descarte e limites são declarados antes de executar; nada é presumido.', parametros);
  const revisao: string[] = [];
  if (draft.assistant?.mode === 'assisted' && !draft.assistant.review_confirmed) revisao.push('Confirmar a revisão da entrevista');
  push('revisao', 'Registrar a revisão', 'A confirmação registra que a hipótese foi conferida antes da execução.', revisao);
  const result = resultRecord(draft.result);
  const execucao: string[] = !result ? ['Executar a simulação com os dados revisados'] : result.status === 'blocked' ? ['Rever as pendências do motor e executar novamente'] : [];
  push('execucao', 'Executar a simulação', 'O motor é local e não transforma pendência em dado válido.', execucao);
  return groups.sort((left, right) => left.step - right.step);
}

export interface JourneyTechnicalLine { label: string; value: string }
export function journeyTechnical(draft: Calculation): JourneyTechnicalLine[] {
  const lines: JourneyTechnicalLine[] = [];
  const cnis = draft.cnis;
  if (cnis) {
    const hash = cnis.metadata.source_sha256;
    lines.push(
      { label: 'Hash SHA-256 da origem', value: hash ? hash.slice(0, 16) + '…' : 'não informado' },
      { label: 'Extrator', value: cnis.metadata.extractor || 'não informado' },
      { label: 'Páginas lidas', value: String(cnis.metadata.page_count) },
      { label: 'Avisos da leitura', value: String(cnis.warnings.length) },
    );
  } else lines.push({ label: 'Origem CNIS', value: 'nenhuma fonte aplicada' });
  lines.push(
    { label: 'Livro de contribuições', value: draft.contribution_review ? (draft.contribution_review.applied ? 'aplicado' : 'preparado, não aplicado') : 'não preparado' },
    { label: 'Resultado financeiro', value: hasFinancialResult(draft) ? 'presente (experimental)' : 'ausente' },
    { label: 'Revisão registrada', value: draft.assistant?.mode === 'assisted' ? (draft.assistant.review_confirmed ? 'sim' : 'não') : 'não se aplica ao editor manual' },
  );
  const result = resultRecord(draft.result);
  lines.push({ label: 'Estado do motor', value: result ? String(result.status ?? 'sem status') : 'não executado' });
  return lines;
}
