import type { Calculation, ContributionRecord, ContributionReview, ManualSalary } from './model';
import { parseMoney } from './model';
const maxCents = 9223372036854775807n;
export function canonicalAmount(value: string): string {
  const normalized = /^(0|[1-9]\d*)\.\d{2}$/.test(value) ? value : parseMoney(value);
  if (!normalized || !/^(0|[1-9]\d*)\.\d{2}$/.test(normalized)) throw Error('Informe valor não negativo com até duas casas decimais.');
  if (cents(normalized) > maxCents) throw Error('Valor excede a precisão monetária suportada.');
  return normalized;
}
export function cents(value: string): bigint {
  if (!/^(0|[1-9]\d*)\.\d{2}$/.test(value)) throw Error('Valor monetário canônico inválido.');
  return BigInt(value.replace('.', ''));
}
export function fromCents(value: bigint): string {
  if (value < 0n || value > maxCents) throw Error('Soma excede o limite monetário.');
  return (value / 100n).toString() + '.' + (value % 100n).toString().padStart(2, '0');
}
export function amountLabel(value: string): string {
  cents(value);
  const [whole, fraction] = value.split('.');
  return whole.replace(/\B(?=(\d{3})+(?!\d))/g, '.') + ',' + fraction;
}
export function invalidateContributions(caseData: Calculation, review: ContributionReview): Calculation {
  return { ...caseData, contribution_review: review, result: null, status: 'Revisão pendente',
    assistant: caseData.assistant ? { ...caseData.assistant, review_confirmed: false,
      completed_steps: caseData.assistant.completed_steps.filter(step => !['review', 'result'].includes(step)) } : null };
}
export function beginContributionReview(caseData: Calculation): Calculation {
  if (caseData.contribution_review) return caseData;
  const records: ContributionRecord[] = (caseData.cnis?.salary_entries ?? []).map(entry => {
    if (!Number.isSafeInteger(entry.amount_centavos) || entry.amount_centavos < 0 || !Number.isSafeInteger(entry.source.index) || entry.source.index < 1)
      throw Error('Origem fora da precisão segura; nenhuma preparação parcial foi aceita.');
    const link = caseData.cnis?.links.find(item => item.sequence === entry.link_sequence);
    const indicators = new Set(entry.indicators);
    if (link?.prpps) indicators.add('PRPPS');
    if (entry.excluded_from_validated_domain && !link?.prpps) indicators.add('origem_excluida_a_revisar');
    return { key: 'source:' + entry.source.index, source_indices: [entry.source.index], competence: entry.competence,
      amount: fromCents(BigInt(entry.amount_centavos)), currency: 'unconfirmed', link_sequence: entry.link_sequence,
      indicators: [...indicators].join(', '), confirmed: false, excluded: false, evidence: '' };
  });
  caseData.manual_salaries.forEach((entry, index) => records.push({ key: 'manual:legacy-' + index, source_indices: [],
    competence: entry.competence, amount: canonicalAmount(entry.amount), currency: 'unconfirmed', link_sequence: null,
    indicators: entry.indicators, confirmed: false, excluded: false, evidence: '' }));
  if (records.length > 10000) throw Error('Livro excede 10.000 lançamentos.');
  return invalidateContributions(caseData, { version: 1, source_sha256: caseData.cnis?.metadata.source_sha256 ?? null,
    records, history: [], applied: false });
}
function checkRecord(record: ContributionRecord) {
  if (!/^[a-zA-Z0-9:._-]{1,100}$/.test(record.key) || !/^(?!0000)\d{4}-(0[1-9]|1[0-2])$/.test(record.competence)) throw Error('Chave ou competência AAAA-MM inválida.');
  if (canonicalAmount(record.amount) !== record.amount) throw Error('Valor deve estar em centavos canônicos.');
  if (!['unconfirmed','BRL','CRUZEIRO','CRUZADO','URV','OTHER'].includes(record.currency)) throw Error('Moeda inválida.');
  if (record.evidence.length > 1000 || record.indicators.length > 1000) throw Error('Texto de revisão excede o limite.');
  if ((record.confirmed || record.excluded) && !record.evidence.trim()) throw Error('Confirmação ou exclusão exige justificativa documental.');
}
export function changeContributions(caseData: Calculation, records: ContributionRecord[], action: string, reason: string): Calculation {
  const previous = caseData.contribution_review;
  if (!previous) throw Error('Prepare o livro antes de alterar.');
  if (!reason.trim() || reason.length > 1000) throw Error('Informe justificativa de até 1.000 caracteres.');
  if (previous.history.length >= 200) throw Error('Histórico atingiu 200 operações; preserve este caso e trabalhe em uma hipótese separada.');
  if (records.length > 10000) throw Error('Livro excede 10.000 lançamentos.');
  records.forEach(checkRecord);
  const old = new Map(previous.records.map(record => [record.key, record]));
  const next = new Map(records.map(record => [record.key, record]));
  if (next.size !== records.length) throw Error('Chave de lançamento duplicada.');
  const changed = new Set([...old.keys(), ...next.keys()].filter(key => JSON.stringify(old.get(key)) !== JSON.stringify(next.get(key))));
  if (!changed.size) return caseData;
  const before = previous.records.filter(record => changed.has(record.key));
  const after = records.filter(record => changed.has(record.key));
  const auditSize = previous.history.reduce((sum, entry) => sum + entry.before.length + entry.after.length, 0) + before.length + after.length;
  if (auditSize > 50000) throw Error('Histórico excede o limite de registros; nenhuma alteração foi descartada.');
  const review = { ...previous, applied: false, records, history: [...previous.history, {
    sequence: previous.history.length + 1, action, reason: reason.trim(), at: new Date().toISOString(), before, after }] };
  return invalidateContributions(caseData, review);
}
export function newRecordKey(review: ContributionReview): string {
  const all = new Set([...review.records, ...review.history.flatMap(entry => [...entry.before, ...entry.after])].map(record => record.key));
  let value = 1; while (all.has('manual:' + value)) value++;
  return 'manual:' + value;
}
export function mergeContributions(caseData: Calculation, keys: string[], reason: string): Calculation {
  const review = caseData.contribution_review!;
  const selected = review.records.filter(record => keys.includes(record.key));
  if (selected.length < 2 || selected.some(record => record.excluded) || new Set(selected.map(record => record.competence)).size !== 1 || new Set(selected.map(record => record.currency)).size !== 1)
    throw Error('Consolide pelo menos dois lançamentos ativos da mesma competência e moeda.');
  const sources = selected.flatMap(record => record.source_indices);
  const merged: ContributionRecord = { ...selected[0], source_indices: sources,
    amount: fromCents(selected.reduce((sum, record) => sum + cents(record.amount), 0n)),
    indicators: [...new Set(selected.map(record => record.indicators).filter(Boolean))].join(', '),
    link_sequence: new Set(selected.map(record => record.link_sequence)).size === 1 ? selected[0].link_sequence : null,
    confirmed: false, evidence: reason.trim() };
  return changeContributions(caseData, review.records.flatMap(record => record.key === merged.key ? [merged] : keys.includes(record.key) ? [] : [record]), 'merge', reason);
}
export function contributionProjection(review: ContributionReview): ManualSalary[] {
  const active = review.records.filter(record => !record.excluded);
  if (!active.length) throw Error('Nenhuma contribuição ativa para aplicar.');
  const months = new Set<string>();
  for (const record of active) {
    checkRecord(record);
    if (!record.confirmed || !record.evidence.trim()) throw Error('Todos os lançamentos ativos exigem conferência e justificativa.');
    if (record.currency !== 'BRL' || record.competence < '1994-07') throw Error('Este motor exige BRL e competências a partir de 1994-07; não há conversão histórica automática.');
    if (record.indicators.trim()) throw Error('Indicadores pendentes impedem aplicar; resolva-os documentalmente.');
    if (months.has(record.competence)) throw Error('Concomitância exige consolidação documental explícita por competência.');
    months.add(record.competence);
  }
  return active.map(record => ({ id: 0, competence: record.competence, amount: record.amount, indicators: record.indicators, confirmed: true }));
}
export function applyContributionReview(caseData: Calculation): Calculation {
  const review = caseData.contribution_review;
  if (!review) throw Error('Prepare o livro.');
  const salaries = contributionProjection(review);
  if (review.history.length >= 200) throw Error('Histórico atingiu 200 operações.');
  const applied = { ...review, applied: true, history: [...review.history, { sequence: review.history.length + 1, action: 'apply', reason: 'Projeção revisada aplicada aos salários; demais parâmetros permanecem sujeitos a revisão.', at: new Date().toISOString(), before: [], after: [] }] };
  return { ...invalidateContributions(caseData, applied), manual_salaries: salaries };
}
export function undoTarget(review: ContributionReview) {
  const undone = new Set(review.history.filter(entry => entry.action === 'undo').map(entry => Number(/^Desfazer operação (\d+):/.exec(entry.reason)?.[1])));
  return [...review.history].reverse().find(entry => !['undo','apply'].includes(entry.action) && !undone.has(entry.sequence));
}
export function undoContributionChange(caseData: Calculation): Calculation {
  const review = caseData.contribution_review!;
  const last = undoTarget(review);
  if (!last) throw Error('Nenhuma operação disponível para desfazer.');
  const changedKeys = new Set([...last.before, ...last.after].map(record => record.key));
  const restored = [...review.records.filter(record => !changedKeys.has(record.key)), ...last.before].sort((a,b) => a.competence.localeCompare(b.competence) || a.key.localeCompare(b.key));
  return changeContributions(caseData, restored, 'undo', 'Desfazer operação ' + last.sequence + ': ' + last.reason);
}
