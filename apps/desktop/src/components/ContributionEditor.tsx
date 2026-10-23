import { useState,useEffect } from 'react';
import type { Calculation, ContributionRecord } from '../model';
import { amountLabel, cents, canonicalAmount, beginContributionReview, changeContributions, mergeContributions, newRecordKey, applyContributionReview, undoContributionChange, undoTarget } from '../contributions';
import { Badge, Button, Field, Icon, Modal, Notice, Panel } from './ui';
const currencies = [['unconfirmed','A confirmar'],['BRL','BRL'],['CRUZEIRO','Cruzeiro'],['CRUZADO','Cruzado'],['URV','URV'],['OTHER','Outra moeda']] as const;
export function ContributionEditor({ draft, setDraft, onNotify, sourceRequest }: {
  sourceRequest?:{month:string;id:number}|null;
  draft: Calculation; setDraft: (value: Calculation) => void; onNotify: (message: string, error?: boolean) => void;
}) {
  const [start, setStart] = useState(''), [end, setEnd] = useState(''), [link, setLink] = useState('all');
  const [status, setStatus] = useState('all'), [selected, setSelected] = useState<string[]>([]);
  const [reason, setReason] = useState(''), [page, setPage] = useState(0), [pageSize, setPageSize] = useState(100);
  const [editing, setEditing] = useState<ContributionRecord | null>(null), [isNew, setIsNew] = useState(false);
  const review = draft.contribution_review;
  const [sourceMonth,setSourceMonth]=useState<string|null>(null);
  useEffect(()=>{if(sourceRequest&&review){setSourceMonth(sourceRequest.month);setStart('');setEnd('');setLink('all');setStatus('all');setSelected([]);setPage(0)}},[sourceRequest?.id]);
  const openEdit=(record:ContributionRecord)=>{setIsNew(false);setSourceMonth(null);setEditing({...record,amount:amountLabel(record.amount),confirmed:false,evidence:''})};
  const perform = (operation: () => Calculation, message: string) => {
    try { setDraft(operation()); setSelected([]); onNotify(message); return true; }
    catch (error) { onNotify(error instanceof Error ? error.message : 'Alteração recusada.', true); return false; }
  };
  const resetFilter = () => { setPage(0); setSelected([]); };
  if (!review) return <Panel title="Livro de contribuições" subtitle="Revise os lançamentos sem alterar o CNIS original.">
    <Notice>O livro mantém origem e ajustes separados. Preparar não confirma moeda, regime, indicadores ou direito a benefício. Salários manuais existentes serão mantidos como lançamentos separados para conferência.</Notice>
    <Button primary onClick={() => perform(() => beginContributionReview(draft), 'Livro preparado sem modificar a origem; revisão pendente.')}><Icon name="file" size={16} />Abrir livro de contribuições</Button>
  </Panel>;
  const sourceMonthByIndex=new Map((draft.cnis?.salary_entries??[]).map(entry=>[entry.source.index,entry.competence]));
  const sourceChanged = review.source_sha256 !== (draft.cnis?.metadata.source_sha256 ?? null);
  const filtered = review.records.filter(record => (!start || record.competence >= start) && (!end || record.competence <= end)
    && (link === 'all' || link === 'none' && record.link_sequence === null || String(record.link_sequence) === link)
    && (status === 'all' || status === 'active' && !record.excluded || status === 'excluded' && record.excluded
      || status === 'pending' && !record.excluded && (!record.confirmed || record.currency === 'unconfirmed' || Boolean(record.indicators.trim()))));
  const pages = Math.max(1, Math.ceil(filtered.length / pageSize));
  const actualPage = Math.min(page, pages - 1), visible = filtered.slice(actualPage * pageSize, (actualPage + 1) * pageSize);
  const chosen = review.records.filter(record => selected.includes(record.key));
  const totals = new Map<string, bigint>();
  for (const record of filtered.filter(row => !row.excluded)) totals.set(record.currency, (totals.get(record.currency) ?? 0n) + cents(record.amount));
  const totalsText = [...totals].map(([currency, total]) => (currencies.find(item => item[0] === currency)?.[1] ?? currency) + ': ' + amountLabel((total / 100n).toString() + '.' + (total % 100n).toString().padStart(2, '0'))).join(' · ');
  const bulk = (action: 'confirm' | 'exclude' | 'restore' | 'currency') => perform(() => changeContributions(draft,
    review.records.map(record => !selected.includes(record.key) ? record : { ...record, evidence: reason.trim(),
      confirmed: action === 'confirm' ? true : false, excluded: action === 'exclude' ? true : action === 'restore' ? false : record.excluded,
      currency: action === 'currency' ? 'BRL' : record.currency }), action, reason), 'Revisão registrada; aplique novamente os salários quando concluir.');
  const originalAmount = (record: ContributionRecord) => {
    if (!record.source_indices.length) return 'Inclusão manual';
    let total = 0n;
    for (const index of record.source_indices) {
      const entry = draft.cnis?.salary_entries.find(item => item.source.index === index);
      if (!entry || !Number.isSafeInteger(entry.amount_centavos)) return 'Origem não disponível';
      total += BigInt(entry.amount_centavos);
    }
    return amountLabel((total / 100n).toString() + '.' + (total % 100n).toString().padStart(2, '0'));
  };
  const allVisibleSelected = visible.length > 0 && visible.every(record => selected.includes(record.key));
  return <Panel title="Livro de contribuições" subtitle="Origem imutável, ajustes documentados e aplicação explícita ao cálculo." className="contribution-panel" actions={<Badge tone={review.applied ? 'good' : 'review'}>{review.applied ? 'Aplicado aos salários' : 'Revisão não aplicada'}</Badge>}>
    {sourceChanged && <Notice tone="error">O documento de origem mudou. Não aplique este livro: preserve esta hipótese e crie outro cálculo para o novo documento.</Notice>}
    <Notice>Valores nominais não são automaticamente corrigidos, convertidos ou validados. Consolidar concomitância exige decisão documental. Aplicar o livro não remove os bloqueios de períodos, regime e exclusão explícita da fonte no adaptador.</Notice>
    <div className="contribution-filters">
      <Field label="Competência inicial" type="month" value={start} onChange={value => { setStart(value); resetFilter(); }} />
      <Field label="Competência final" type="month" value={end} onChange={value => { setEnd(value); resetFilter(); }} />
      <label className="field"><span>Vínculo de origem</span><select className="input" aria-label="Vínculo de origem" value={link} onChange={event => { setLink(event.target.value); resetFilter(); }}><option value="all">Todos os vínculos</option><option value="none">Sem vínculo</option>{draft.cnis?.links.map(item => <option key={item.sequence} value={item.sequence}>Vínculo {item.sequence}{(draft.manual_links.find(link=>link.source_review?.sequence===item.sequence)?.employer??item.raw_employer) ? ' · ' + (draft.manual_links.find(link=>link.source_review?.sequence===item.sequence)?.employer??item.raw_employer) : ''}</option>)}</select></label>
      <label className="field"><span>Situação dos lançamentos</span><select className="input" aria-label="Situação dos lançamentos" value={status} onChange={event => { setStatus(event.target.value); resetFilter(); }}><option value="all">Todos</option><option value="active">Ativos</option><option value="pending">Pendentes</option><option value="excluded">Excluídos da hipótese</option></select></label>
    </div>
    <p className="contribution-summary" role="status">{filtered.length} lançamentos · {new Set(filtered.map(record => record.competence)).size} competências · {selected.length} selecionados</p>
    <p className="secondary-line">Somas nominais ativas no filtro, separadas por moeda: {totalsText || 'Sem valores ativos'}</p>
    <div className="contribution-actions" data-ui-actions>
      <Button disabled={sourceChanged} onClick={() => { setIsNew(true); setEditing({ key: newRecordKey(review), source_indices: [], competence: '', amount: '', currency: 'unconfirmed', link_sequence: null, indicators: '', confirmed: false, excluded: false, evidence: '', amount_kind: 'remuneration' }); }}>Incluir contribuição</Button>
      <Button disabled={sourceChanged || !undoTarget(review)} onClick={() => perform(() => undoContributionChange(draft), 'Última operação desfeita; o resultado anterior não foi reutilizado.')}>Desfazer última alteração</Button>
      <Button primary disabled={sourceChanged} onClick={() => perform(() => applyContributionReview(draft), 'Livro aplicado aos salários. Revise os demais parâmetros antes de executar o motor.')}>Aplicar revisão ao cálculo</Button>
    </div>
    <div className="table-wrap contribution-table" tabIndex={0} role="region" aria-label="Tabela de contribuições; role horizontalmente para todas as colunas">
      <table className="data-table"><thead><tr><th><input type="checkbox" aria-label="Selecionar lançamentos desta página" checked={allVisibleSelected} disabled={sourceChanged || !visible.length} onChange={event => setSelected(event.target.checked ? [...new Set([...selected, ...visible.map(record => record.key)])] : selected.filter(key => !visible.some(record => record.key === key)))} /></th><th>Competência</th><th>Origem / vínculo</th><th className="num">Valor original</th><th className="num">Valor ajustado</th><th>Moeda</th><th>Indicadores / revisão</th><th>Ações</th></tr></thead>
        <tbody>{visible.map(record => <tr key={record.key} data-contribution-key={record.key} className={'source-edit-row '+(record.excluded?'contribution-excluded':'')} onClick={()=>{if(!sourceChanged)openEdit(record)}}><td onClick={event=>event.stopPropagation()}><input type="checkbox" aria-label={'Selecionar contribuição ' + record.key} disabled={sourceChanged} checked={selected.includes(record.key)} onChange={event => setSelected(event.target.checked ? [...selected, record.key] : selected.filter(key => key !== record.key))} /></td><td><Button className="source-cell" aria-label={'Editar competência da contribuição '+record.key} disabled={sourceChanged} onClick={event=>{event.stopPropagation();openEdit(record)}}>{record.competence}</Button>{record.source_indices.length > 0 && <div className="secondary-line">Origem: {[...new Set(record.source_indices.map(index => draft.cnis?.salary_entries.find(item => item.source.index === index)?.competence ?? 'indisponível'))].join(', ')}</div>}</td><td>{record.source_indices.length ? 'CNIS #' + record.source_indices.join(', #') : 'Manual'}<div className="secondary-line">{record.link_sequence === null ? 'Sem vínculo único' : 'Vínculo ' + record.link_sequence}</div>{record.amount_kind && record.amount_kind !== 'remuneration' && <div className="secondary-line">{record.amount_kind === 'contribution_column_unresolved' ? 'Valor de contribuição sem salário identificado' : 'Valor consolidado por ano civil'}</div>}</td><td className="num">{originalAmount(record)}</td><td className="num"><Button className="source-cell" disabled={sourceChanged} aria-label={'Editar valor da contribuição '+record.key} onClick={event=>{event.stopPropagation();openEdit(record)}}>{amountLabel(record.amount)}</Button></td><td>{currencies.find(item => item[0] === record.currency)?.[1] ?? record.currency}</td><td><Badge tone={record.excluded ? 'neutral' : record.confirmed ? 'good' : 'review'}>{record.excluded ? 'Excluído da hipótese' : record.confirmed ? 'Conferência declarada' : 'Pendente'}</Badge><div className="secondary-line">{record.indicators || 'Sem indicadores declarados'}</div></td><td><Button disabled={sourceChanged} aria-label={'Editar contribuição ' + record.key} onClick={event=>{event.stopPropagation();openEdit(record)}}>Editar</Button></td></tr>)}</tbody>
      </table>{!visible.length && <p className="form-note">Nenhum lançamento neste filtro.</p>}
    </div>
    <div className="contribution-pagination" data-ui-actions><Button disabled={actualPage === 0} onClick={() => setPage(actualPage - 1)}>Página anterior</Button><span>Página {actualPage + 1} de {pages}</span><Button disabled={actualPage + 1 >= pages} onClick={() => setPage(actualPage + 1)}>Próxima página</Button><label>Linhas por página <select aria-label="Linhas por página" className="input" value={pageSize} onChange={event => { setPageSize(Number(event.target.value)); setPage(0); }}><option value="50">50</option><option value="100">100</option><option value="250">250</option></select></label></div>
    <div className="contribution-bulk"><Field required label="Justificativa documental da ação em lote" value={reason} onChange={setReason} placeholder="Referência do documento e motivo; não basta marcar um botão" /><div className="contribution-actions" data-ui-actions><Button disabled={sourceChanged || !chosen.length} onClick={() => bulk('currency')}>Declarar BRL nos selecionados</Button><Button disabled={sourceChanged || !chosen.length || chosen.some(record => record.excluded)} onClick={() => bulk('confirm')}>Conferir selecionados</Button><Button disabled={sourceChanged || !chosen.length} onClick={() => bulk('exclude')}>Excluir selecionados da hipótese</Button><Button disabled={sourceChanged || !chosen.length} onClick={() => bulk('restore')}>Reincluir selecionados</Button><Button disabled={sourceChanged || chosen.length < 2} onClick={() => perform(() => mergeContributions(draft, selected, reason), 'Concomitância consolidada nominalmente; conferência necessária.')}>Consolidar competência selecionada</Button></div></div>
    <details className="details"><summary>Histórico de revisão · {review.history.length} operações</summary><p className="form-note">Trilha local, não assinatura digital. A origem e os valores anteriores são preservados; nenhum resultado antigo é restaurado ao desfazer.</p>{review.history.map(entry => <details key={entry.sequence} className="contribution-history"><summary>#{entry.sequence} · {entry.action} · {entry.at}</summary><p>{entry.reason}</p><pre className="result-json">{JSON.stringify({ before: entry.before, after: entry.after }, null, 2)}</pre></details>)}</details>
    {sourceMonth&&<Modal title={'Editar competência '+sourceMonth} subtitle="Escolha o lançamento para ajustar. A soma não sobrescreve contribuições individuais ou moedas históricas." wide onClose={()=>setSourceMonth(null)}>
      <Notice>Todos os lançamentos originais desta competência continuam no CNIS. Clique no valor, no vínculo ou na revisão para editar o ajuste correspondente; nada é confirmado automaticamente.</Notice>
      <div className="table-wrap source-edit-table"><table className="data-table"><thead><tr><th>Competência ajustada</th><th>Origem / vínculo</th><th>Valor original</th><th>Valor ajustado</th><th>Situação</th><th>Ação</th></tr></thead><tbody>{review.records.filter(record=>record.source_indices.some(index=>sourceMonthByIndex.get(index)===sourceMonth)).map(record=><tr key={record.key} className="source-edit-row" onClick={()=>openEdit(record)}><td>{record.competence}</td><td>{record.source_indices.map(index=>'CNIS #'+index).join(', ')} · {record.link_sequence===null?'Sem vínculo único':'Vínculo '+record.link_sequence}</td><td>{originalAmount(record)}</td><td><Button className="source-cell" aria-label={'Editar lançamento '+record.key+' da competência '+sourceMonth} onClick={event=>{event.stopPropagation();openEdit(record)}}>{amountLabel(record.amount)}</Button></td><td>{record.excluded?'Excluído da hipótese':record.confirmed?'Conferência declarada':'Pendente'}</td><td><Button onClick={event=>{event.stopPropagation();openEdit(record)}} aria-label={'Editar contribuição '+record.key}>Editar</Button></td></tr>)}</tbody></table></div>
    </Modal>}
    {editing && <Modal title={isNew ? 'Incluir contribuição' : 'Editar contribuição'} subtitle="Ajuste somente a hipótese; o lançamento original permanece no CNIS." onClose={() => setEditing(null)}>
      <div className="form-grid"><Field required label="Competência AAAA-MM" type="month" value={editing.competence} onChange={competence => setEditing({ ...editing, competence })} /><Field required label="Valor nominal ajustado" value={editing.amount} onChange={amount => setEditing({ ...editing, amount })} />
        <label className="field"><span>Moeda declarada</span><select className="input" aria-label="Moeda declarada" value={editing.currency} onChange={event => setEditing({ ...editing, currency: event.target.value })}>{currencies.map(([value,label]) => <option key={value} value={value}>{label}</option>)}</select></label>
        <label className="field"><span>Vínculo declarado</span><select className="input" aria-label="Vínculo declarado" value={editing.link_sequence ?? ''} onChange={event => setEditing({ ...editing, link_sequence: event.target.value ? Number(event.target.value) : null })}><option value="">Sem vínculo único</option>{draft.cnis?.links.map(item => <option key={item.sequence} value={item.sequence}>Vínculo {item.sequence}</option>)}</select></label>
        <Field label="Indicadores pendentes" value={editing.indicators} onChange={indicators => setEditing({ ...editing, indicators })} />
        <Field required label="Justificativa documental do ajuste" value={editing.evidence} onChange={evidence => setEditing({ ...editing, evidence })} />
      </div><label className="checkline"><input type="checkbox" checked={editing.confirmed} onChange={event => setEditing({ ...editing, confirmed: event.target.checked })} />Conferência documental deste lançamento</label>
      <div className="form-actions" data-ui-actions><Button onClick={() => setEditing(null)}>Cancelar</Button><Button primary onClick={() => {
        if (perform(() => {
          const record = { ...editing, amount: canonicalAmount(editing.amount) };
          return changeContributions(draft, isNew ? [...review.records, record] : review.records.map(item => item.key === record.key ? record : item), isNew ? 'add' : 'edit', editing.evidence);
        }, 'Ajuste documentado; resultado e revisão anterior invalidados.')) setEditing(null);
      }}>Salvar ajuste</Button></div>
    </Modal>}
  </Panel>;
}
