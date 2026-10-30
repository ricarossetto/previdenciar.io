import type {AssistantInterview, Calculation, CnisLink, ManualLink} from './model';
import {isoDate, parseMoney, robotParameterIssues, hasFinancialResult} from './model';

/** Versioned policy accepted by the local RGPS adapter. */
export const POLICY_VERSION = 'rgps-core-observational-v1';
const LINK_KINDS = ['empregado','empregado_domestico','contribuinte_individual','avulso','facultativo'];

export type ConsiderationFix = 'interview'|'source'|'links'|'salaries'|'policy'|'parameters';
export interface ConsiderationIssue {
  code: string;
  label: string;
  /** True when the current input would be refused or left without a financial result. */
  decisive: boolean;
  fix: ConsiderationFix;
}
export interface AssistantConsideration {
  imported: {
    present: boolean;
    links: number;
    entries: number;
    months: number;
    prpps: number;
    missingEnd: number;
    warnings: number;
    identity: boolean;
  };
  adjusted: {
    manualLinks: number;
    selectedLinks: number;
    confirmedLinks: number;
    excludedLinks: number;
    editedSourceLinks: number;
    salaries: number;
    confirmedSalaries: number;
    contributionRecords: number;
    contributionExcluded: number;
    contributionApplied: boolean;
    sourceExcluded: boolean;
    sourceExclusionDocumented: boolean;
    reviewEvidence: boolean;
    policyVersion: string|null;
  };
  selected: {links:number; salaries:number; months:number};
  used: {links:number; salaries:number; months:number; source:boolean};
  issues: ConsiderationIssue[];
  sourceNotes: ConsiderationIssue[];
  effect: string;
  conclusion: string;
  ready: boolean;
  execution: 'pending'|'ready'|'blocked'|'calculated';
}

function object(value: unknown): Record<string, unknown>|null {
  return value !== null && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : null;
}
function text(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}
function policyOf(c: Calculation): Record<string, unknown>|null {
  const raw = c.options.rule_policy.trim();
  if (!raw.startsWith('{')) return null;
  try { return object(JSON.parse(raw)); } catch { return null; }
}
function cents(value: string): bigint|null {
  const canonical = /^(0|[1-9]\d*)\.(\d{1,2})$/.exec(value.trim());
  const normalized = canonical ? canonical[1] + '.' + canonical[2].padEnd(2,'0') : parseMoney(value);
  if (!normalized || !/^(0|[1-9]\d*)\.\d{2}$/.test(normalized)) return null;
  return BigInt(normalized.replace('.',''));
}
function positiveMoney(value: string): boolean {
  const valueCents = cents(value);
  return valueCents !== null && valueCents > 0n;
}
function validCompetence(value: string): boolean {
  return /^(19|20)\d{2}-(0[1-9]|1[0-2])$/.test(value);
}
function dateRange(link: ManualLink): {start:string; end:string}|null {
  const start = isoDate(link.start_date);
  const end = isoDate(link.end_date);
  return start && end ? {start,end} : null;
}
function monthInside(competence: string, start: string, end: string): boolean {
  const month = competence.slice(0,7);
  return month >= start.slice(0,7) && month <= end.slice(0,7);
}
function nonEmpty(value: unknown): boolean {
  return typeof value === 'string' && value.trim().length > 0;
}
function selectedLinks(c: Calculation): ManualLink[] {
  return c.manual_links.filter(link => !link.source_review?.excluded);
}
function identityConflict(c: Calculation): boolean {
  const cnis = c.cnis;
  if (!cnis) return false;
  return [
    [c.identity.full_name, cnis.identity.full_name],
    [c.identity.cpf, cnis.identity.cpf],
    [c.identity.birth_date, cnis.identity.birth_date],
  ].some(([a,b]) => nonEmpty(a) && nonEmpty(b) && a !== b);
}
function sourceFindings(c: Calculation): ConsiderationIssue[] {
  const notes: ConsiderationIssue[] = [];
  const cnis = c.cnis;
  if (!cnis) return notes;
  if (cnis.links.some(link => link.prpps)) notes.push({code:'source_prpps',label:'A fonte contém vínculo PRPPS/RPPS; não é promovido a RGPS.',decisive:false,fix:'source'});
  if (cnis.links.some(link => !link.end_date)) notes.push({code:'source_end_missing',label:'A fonte contém vínculo sem data final; o último salário não prova o encerramento.',decisive:false,fix:'source'});
  if (cnis.salary_entries.some(entry => entry.link_sequence === null)) notes.push({code:'source_unassigned',label:'A fonte contém remuneração sem vínculo reconhecido.',decisive:false,fix:'source'});
  if (cnis.warnings.length || cnis.links.some(link => link.indicators.length) || cnis.salary_entries.some(entry => entry.indicators.length)) notes.push({code:'source_indicators',label:'A fonte preserva indicadores/avisos que exigem conferência documental.',decisive:false,fix:'source'});
  return notes;
}

/**
 * Separates what came from the source, what the operator changed and what the
 * reviewed RGPS input would actually send to the engine. The CNIS source is
 * never promoted to calculation input by this app; only reviewed manual rows
 * are. A missing item stays missing and a blocked scenario is not financial.
 */
export function assistantConsideration(c: Calculation, _interview?: AssistantInterview): AssistantConsideration {
  const a = _interview ?? c.assistant;
  const policy = policyOf(c);
  const native = object(policy?.native_options);
  const sourceExcluded = policy?.exclude_imported_source === true;
  const sourceExclusionDocumented = sourceExcluded && nonEmpty(policy?.source_exclusion_evidence);
  const reviewEvidence = nonEmpty(policy?.review_evidence);
  const policyVersion = text(policy?.version) || null;
  const links = selectedLinks(c);
  const salaries = c.manual_salaries;
  const months = new Set(salaries.map(s => s.competence));
  const issues: ConsiderationIssue[] = [];
  const seen = new Set<string>();
  function add(code: string, label: string, fix: ConsiderationFix, decisive = true) {
    if (seen.has(code)) return;
    seen.add(code);
    issues.push({code,label,fix,decisive});
  }

  if (!a || a.mode !== 'assisted') add('interview_missing','Entrevista assistida não registrada.', 'interview');
  else {
    if (a.current_benefit_received === null) add('benefit_unconfirmed','Situação de benefício atual não confirmada.','interview');
    if (a.current_benefit_received === true) add('benefit_received','Revisão/restabelecimento a partir de benefício recebido não são suportados.','interview');
    if (!a.process_kind) add('process_missing','Finalidade do cálculo não informada.','interview');
    if (!a.goal) add('goal_missing','Objetivo do cálculo não informado.','interview');
    else if (a.goal !== 'Calcular um novo benefício') add('goal_unsupported','Objetivo registrado sem cálculo automático neste motor.','interview');
  }
  for (const label of robotParameterIssues(c)) add('parameter:' + label, 'Falta confirmar: ' + label + '.', 'parameters');
  if (!c.name.trim()) add('case_name_missing','O cálculo ainda não tem nome para ser salvo.','interview');

  if (c.cnis && !sourceExcluded) add('source_gate','Fonte CNIS ainda não excluída com justificativa: o motor não calcula sobre a fonte importada.','source');
  if (sourceExcluded && !sourceExclusionDocumented) add('source_exclusion_evidence','A exclusão da fonte não tem referência documental.','source');

  if (!policy) add('policy_missing','Política versionada ausente ou ilegível.','policy');
  else {
    if (policyVersion !== POLICY_VERSION) add('policy_version','Política fora da versão suportada pelo motor.','policy');
    if (!reviewEvidence) add('review_evidence','Referência documental da revisão ausente.','policy');
    if (!LINK_KINDS.includes(text(policy.manual_link_kind))) add('manual_link_kind','Categoria dos períodos manuais não declarada.','policy');
    if (salaries.length > 0 && text(policy.manual_currency) !== 'BRL') add('manual_currency','Moeda dos salários manuais não declarada como BRL.','policy');
    const optionKeys = ['corrigirSalarios','descarte','divisorMinimo','aplicarTetoVigenteDer'];
    const missingOptions = optionKeys.filter(key => typeof native?.[key] !== 'boolean');
    if (missingOptions.length) add('policy_options','Opções de correção/descarte/divisor/teto não declaradas.','policy');
    if (native?.corrigirSalarios === true && !nonEmpty(native?.referenciaCorrecao)) add('correction_reference','Correção ativa sem competência de referência.','policy');
    if (native?.corrigirSalarios === true && native?.divisorMinimo === true) add('corrected_divisor','Divisor mínimo com correção ativa é uma combinação bloqueada.','policy');
    if (native?.corrigirSalarios === false && native?.descarte === true) add('uncorrected_discard','Descarte sem correção ativa é uma combinação bloqueada.','policy');
    if (native?.aplicarTetoVigenteDer === true && nonEmpty(native?.teto)) add('ceiling_conflict','Teto manual junto ao teto derivado é uma combinação bloqueada.','policy');
    if (nonEmpty(native?.expectativaSobrevida)) add('fator_unvalidated','Expectativa de sobrevida declarada aciona um caminho de fator ainda bloqueado.','policy');
    const ceiling = nonEmpty(native?.teto) ? cents(String(native?.teto)) : null;
    const floor = nonEmpty(native?.piso) ? cents(String(native?.piso)) : null;
    if (nonEmpty(native?.teto) && ceiling === null) add('ceiling_invalid','Teto manual não é um valor monetário válido.','policy');
    if (nonEmpty(native?.piso) && floor === null) add('floor_invalid','Piso manual não é um valor monetário válido.','policy');
    if (ceiling !== null && floor !== null && floor > ceiling) add('limits_inverted','Piso manual maior que o teto manual.','policy');
  }
  if (c.options.correction_index.trim() && !c.options.correction_index.trim().toLowerCase().includes('inpc')) add('correction_index','Índice de correção diferente de INPC não é suportado neste caminho.','policy');
  if (c.contribution_review && !c.contribution_review.applied) add('contribution_unapplied','Livro de contribuições alterado e ainda não aplicado aos salários.','salaries');
  if (identityConflict(c)) add('identity_conflict','Identidade do cálculo diverge da identidade importada.','parameters');

  if (!c.cnis && !c.manual_links.length && !c.manual_salaries.length) add('no_data','Sem CNIS ou registros manuais para o motor.','links');
  if (links.length === 0) add('links_missing','Nenhum vínculo manual selecionado para o motor.','links');
  const unconfirmedLinks = links.filter(link => !link.confirmed).length;
  if (unconfirmedLinks) add('links_unconfirmed',unconfirmedLinks + ' vínculo(s) sem confirmação documental.','links');
  const nonRgps = links.filter(link => link.regime.trim().toUpperCase() !== 'RGPS').length;
  if (nonRgps) add('links_regime',nonRgps + ' vínculo(s) com regime diferente de RGPS.','links');
  if (links.some(link => link.regime.trim().toUpperCase() === 'PRPPS' || link.regime.trim().toUpperCase() === 'RPPS')) add('links_prpps','PRPPS/RPPS exige CTC documental; editar o regime não comprova averbação.','links');
  if (links.some(link => link.indicators.trim().length > 0)) add('links_indicators','Indicadores de vínculo não resolvidos.','links');
  if (links.some(link => !dateRange(link))) add('links_dates','Vínculo sem início e fim válidos.','links');
  const reversed = links.filter(link => { const range = dateRange(link); return range ? range.start > range.end : false; });
  if (reversed.length) add('links_reversed','Vínculo com fim anterior ao início.','links');
  const dib = isoDate(c.dib);
  if (dib && links.some(link => { const range = dateRange(link); return range ? range.end > dib : false; })) add('links_after_dib','Vínculo termina depois da DIB.','links');
  const birth = isoDate(c.identity.birth_date ?? '');
  if (birth && links.some(link => { const range = dateRange(link); return range ? range.start < birth : false; })) add('links_before_birth','Vínculo começa antes do nascimento.','links');
  const ordered = links.map(link => ({link,range:dateRange(link)})).filter((item): item is {link:ManualLink;range:{start:string;end:string}} => item.range !== null).sort((x,y) => x.range.start.localeCompare(y.range.start));
  if (ordered.some((item,index) => index > 0 && ordered[index-1].range.end >= item.range.start)) add('links_overlap','Períodos concomitantes ou sobrepostos: o motor soma apenas a parcela não sobreposta e avisa no resultado.','links',false);

  if (salaries.length === 0) add('salaries_missing','Nenhuma remuneração manual selecionada para o motor.','salaries');
  const unconfirmedSalaries = salaries.filter(s => !s.confirmed).length;
  if (unconfirmedSalaries) add('salaries_unconfirmed',unconfirmedSalaries + ' remuneração(ões) sem confirmação documental.','salaries');
  if (salaries.some(s => s.indicators.trim().length > 0)) add('salaries_indicators','Indicadores de remuneração não resolvidos.','salaries');
  if (salaries.some(s => !validCompetence(s.competence))) add('salaries_competence','Competência de remuneração ausente ou inválida.','salaries');
  if (salaries.some(s => !positiveMoney(s.amount))) add('salaries_amount','Valor de remuneração ausente, não positivo ou inválido.','salaries');
  const competences = salaries.map(s => s.competence).filter(validCompetence);
  if (new Set(competences).size !== competences.length) add('salaries_duplicate','Competência duplicada: concomitância exige consolidação documental.','salaries');
  if (dib && competences.some(month => month > dib.slice(0,7))) add('salaries_after_dib','Existe remuneração posterior à DIB.','salaries');
  if (links.length && competences.some(month => !links.some(link => { const range = dateRange(link); return range ? monthInside(month,range.start,range.end) : false; }))) add('salaries_outside_links','Existe remuneração sem vínculo confirmado correspondente.','salaries');
  const pre94 = competences.filter(month => month < '1994-07').length;
  if (pre94) add('salaries_pre94',pre94 + ' competência(s) anterior(es) a 07/1994 serão preservadas, mas ficam fora do PBC.', 'salaries', false);

  for (const note of sourceFindings(c)) {
    if (note.decisive) add(note.code,note.label,note.fix);
    else if (c.cnis && sourceExcluded) issues.push(note);
  }

  const decisive = issues.filter(item => item.decisive);
  const ready = decisive.length === 0;
  const selected = {links:links.length,salaries:salaries.length,months:months.size};
  const contributionExcluded = c.contribution_review?.records.filter(record => record.excluded).length ?? 0;
  const excludedLinks = c.manual_links.filter(link => link.source_review?.excluded).length;
  const editedSourceLinks = c.manual_links.filter(link => (link.source_review?.history.length ?? 0) > 0).length;
  const effectParts: string[] = [];
  if (editedSourceLinks) effectParts.push(editedSourceLinks + ' vínculo(s) editado(s) com histórico');
  if (excludedLinks) effectParts.push(excludedLinks + ' vínculo(s) excluído(s) da hipótese');
  if (c.contribution_review) effectParts.push(c.contribution_review.records.length + ' lançamento(s) no livro' + (contributionExcluded ? ', ' + contributionExcluded + ' excluído(s)' : ''));
  if (c.contribution_review?.applied) effectParts.push('projeção aplicada aos salários');
  if (sourceExcluded) effectParts.push('fonte CNIS excluída do motor com justificativa registrada');
  const effect = effectParts.length ? effectParts.join('; ') + '.' : 'Nenhum ajuste registrado além da leitura da fonte.';

  const blockedResult = Boolean(c.result && (c.result as Record<string, unknown>).status === 'blocked');
  const experimental = Boolean(c.result && (c.result as Record<string, unknown>).status === 'experimental');
  let conclusion: string;
  if (blockedResult) conclusion = 'A última execução foi bloqueada; nenhum valor financeiro foi produzido.';
  else if (ready && experimental) conclusion = 'O motor recebeu ' + selected.links + ' vínculo(s) e ' + selected.salaries + ' remuneração(ões) revisadas. Resultado experimental disponível; a fonte CNIS não entrou no cálculo.';
  else if (ready) conclusion = 'Os dados revisados estão prontos para o motor: ' + selected.links + ' vínculo(s) e ' + selected.salaries + ' remuneração(ões). A fonte CNIS não entra no cálculo.';
  else conclusion = 'Ainda não é possível calcular: ' + decisive.slice(0,2).map(item => item.label.replace(/\.$/,'')).join('; ') + (decisive.length > 2 ? '; e mais ' + (decisive.length-2) + ' pendência(s).' : '.');

  return {
    execution: blockedResult ? 'blocked' : hasFinancialResult(c) ? 'calculated' : ready ? 'ready' : 'pending',
    imported: {
      present: Boolean(c.cnis),
      links: c.cnis?.links.length ?? 0,
      entries: c.cnis?.salary_entries.length ?? 0,
      months: c.cnis ? Object.keys(c.cnis.salary_months).length : 0,
      prpps: c.cnis?.links.filter(link => link.prpps).length ?? 0,
      missingEnd: c.cnis?.links.filter(link => !link.end_date).length ?? 0,
      warnings: c.cnis?.warnings.length ?? 0,
      identity: Boolean(c.cnis && (c.cnis.identity.full_name || c.cnis.identity.cpf || c.cnis.identity.birth_date)),
    },
    adjusted: {
      manualLinks: c.manual_links.length,
      selectedLinks: links.length,
      confirmedLinks: links.filter(link => link.confirmed).length,
      excludedLinks,
      editedSourceLinks,
      salaries: salaries.length,
      confirmedSalaries: salaries.filter(s => s.confirmed).length,
      contributionRecords: c.contribution_review?.records.length ?? 0,
      contributionExcluded,
      contributionApplied: c.contribution_review?.applied === true,
      sourceExcluded,
      sourceExclusionDocumented,
      reviewEvidence,
      policyVersion,
    },
    selected,
    used: ready ? {links:selected.links,salaries:selected.salaries,months:selected.months,source:false} : {links:0,salaries:0,months:0,source:false},
    issues,
    sourceNotes: sourceFindings(c),
    effect,
    conclusion,
    ready,
  };
}

/** First-link quality data that can be shown without pretending to calculate grace periods. */
export function linkQuality(cnis: {links: CnisLink[]; salary_entries: {competence:string;link_sequence:number|null}[]}): {
  sequence:number; affiliation:string|null; firstCompetence:string|null; lastCompetence:string|null; entries:number; missingEnd:boolean; prpps:boolean; indicators:string[];
}[] {
  return cnis.links.map(link => {
    const entries = cnis.salary_entries.filter(entry => entry.link_sequence === link.sequence).map(entry => entry.competence).sort();
    return {
      sequence: link.sequence,
      affiliation: link.affiliation ?? null,
      firstCompetence: entries[0] ?? null,
      lastCompetence: entries[entries.length-1] ?? null,
      entries: entries.length,
      missingEnd: !link.end_date,
      prpps: link.prpps,
      indicators: link.indicators,
    };
  });
}
