import {useId, useState} from 'react';
import type {Cnis} from '../model';
import {brDate, centavosMoney} from '../model';
import {linkQuality} from '../assistantReview';
import {Button, Notice} from './ui';

const VIEWS = ['Dados pessoais', 'Qualidade', 'Períodos', 'Remunerações'] as const;
const PAGE_SIZE = 20;

/** A preview of source records. Adjustments belong to the review overlay after applying. */
export function CnisReview({cnis}: {cnis:Cnis}) {
  const id = useId();
  const [view, setView] = useState(0);
  const [page, setPage] = useState(0);
  const rows = view === 2 ? cnis.links : view === 3 ? cnis.salary_entries : [];
  const pageCount = Math.max(1, Math.ceil(rows.length / PAGE_SIZE));
  const currentPage = Math.min(page, pageCount - 1);
  function choose(index:number) { setView(index); setPage(0); }
  const quality = view === 1 ? linkQuality(cnis) : [];
  return <section className="cnis-review" aria-label="Conferir dados do CNIS antes de aplicar">
    <div className="cnis-review__tabs" role="tablist" aria-label="Dados importados">{VIEWS.map((label,index)=><button
      key={label} type="button" role="tab" id={`${id}-tab-${index}`} aria-controls={`${id}-panel`}
      aria-selected={view===index} tabIndex={view===index?0:-1} onClick={()=>choose(index)}
      onKeyDown={event=>{let next=index;if(event.key==='ArrowRight')next=(index+1)%VIEWS.length;else if(event.key==='ArrowLeft')next=(index+VIEWS.length-1)%VIEWS.length;else if(event.key==='Home')next=0;else if(event.key==='End')next=VIEWS.length-1;else return;event.preventDefault();choose(next);document.getElementById(`${id}-tab-${next}`)?.focus();}}
    >{label}{index>0&&<span>{index===3?cnis.salary_entries.length:cnis.links.length}</span>}</button>)}</div>
    <div id={`${id}-panel`} role="tabpanel" aria-labelledby={`${id}-tab-${view}`} tabIndex={0}>
      {view===0?<dl className="cnis-review__identity">{[
        ['Nome',cnis.identity.full_name],['Nascimento',brDate(cnis.identity.birth_date)],
        ['CPF',cnis.identity.cpf],['NIT',cnis.identity.nit],
      ].map(([label,value])=><div key={label}><dt>{label}</dt><dd>{value||'Não informado no documento'}</dd></div>)}</dl>
      :view===1?<>
        <div className="table-wrap"><table className="data-table"><caption>Qualidade de segurado: dados por vínculo, sem cálculo de graça ou prorrogação</caption>
          <thead><tr>{['Vínculo','Filiação','Primeiro lançamento','Último lançamento','Lançamentos','Desconsideradas','Fim original','Situação'].map(label=><th key={label} scope="col">{label}</th>)}</tr></thead>
          <tbody>{quality.map(item=>{const excluded=cnis.salary_entries.filter(entry=>entry.link_sequence===item.sequence&&entry.excluded_from_validated_domain).length;return <tr key={item.sequence}>
            <td><strong>Vínculo {item.sequence}</strong></td>
            <td>{item.affiliation||'Não informada no documento'}</td>
            <td>{item.firstCompetence||'Sem lançamento'}</td>
            <td>{item.lastCompetence||'Sem lançamento'}</td>
            <td>{item.entries}</td>
            <td>{excluded?'Sim':'Não'}</td>
            <td>{item.missingEnd?'Fim não informado':'Fim informado'}</td>
            <td>{item.prpps?'PRPPS / CTC pendente':item.indicators.length?'Indicadores pendentes':'Conferência pendente'}</td>
          </tr>})}</tbody></table></div>
        {!quality.length&&<p className="form-note">Nenhum vínculo foi extraído do documento.</p>}
        <p className="form-note">Período de graça, prorrogações e enquadramento não são calculados nesta versão. A tabela reúne filiação, primeiro e último lançamento, contribuições desconsideradas e pendências para conferência por vínculo. Ausência de lançamento não prova ausência de filiação.</p>
      </>
      :<>
        <div className="table-wrap"><table className="data-table"><caption>{view===2?'Períodos informados no documento':'Remunerações nominais do documento: lançamentos separados, sem conversão presumida'}</caption>
          <thead><tr>{(view===2?['Vínculo','Início','Fim','Indicadores','Origem']:['Competência','Valor nominal','Vínculo','Indicadores','Origem']).map(label=><th key={label} scope="col">{label}</th>)}</tr></thead>
          <tbody>{view===2?cnis.links.slice(currentPage*PAGE_SIZE,(currentPage+1)*PAGE_SIZE).map((link,index)=><tr key={`${link.sequence}-${index}`}>
            <td><strong>{link.raw_employer||`Vínculo ${link.sequence}`}</strong>{link.affiliation&&<small>Filiação: {link.affiliation}</small>}{link.prpps&&<small>RPPS / CTC a conferir</small>}</td>
            <td>{brDate(link.start_date)}</td><td>{link.end_date?brDate(link.end_date):'Fim não informado'}{!link.end_date&&link.inferred_last_salary_end&&<small>Último salário: {link.reported_last_salary_month||'não informado'}. Não confirma encerramento.</small>}</td>
            <td>{link.indicators.join(', ')||'Sem indicador informado'}</td><td>Página {link.source.page}</td>
          </tr>):cnis.salary_entries.slice(currentPage*PAGE_SIZE,(currentPage+1)*PAGE_SIZE).map((entry,index)=><tr key={`${entry.source.index}-${index}`}>
            <td>{entry.competence}</td><td className="cnis-review__amount">{centavosMoney(entry.amount_centavos)}</td><td>{entry.link_sequence??'Não associado'}</td>
            <td>{entry.indicators.join(', ')||'Sem indicador informado'}{entry.excluded_from_validated_domain&&<small>Exige revisão antes do cálculo</small>}</td><td>Página {entry.source.page}, linha {entry.source.line}</td>
          </tr>)}</tbody></table></div>
        {!rows.length&&<p className="form-note">Nenhum registro deste tipo foi extraído.</p>}
        {rows.length>PAGE_SIZE&&<div className="cnis-review__pagination"><Button disabled={currentPage===0} onClick={()=>setPage(currentPage-1)}>Página anterior</Button><span role="status">{currentPage*PAGE_SIZE+1} a {Math.min((currentPage+1)*PAGE_SIZE,rows.length)} de {rows.length}</span><Button disabled={currentPage+1>=pageCount} onClick={()=>setPage(currentPage+1)}>Próxima página</Button></div>}
      </>}
    </div>
    {cnis.warnings.length>0&&<details className="details"><summary>Pendências da leitura · {cnis.warnings.length}</summary><ul className="review-list">{cnis.warnings.map((warning,index)=><li key={index}>{warning.message||'Conferência documental necessária'} · {warning.count} registro(s)</li>)}</ul></details>}
    <Notice>Esta é a leitura do documento. Após aplicar, confira e edite vínculos em Períodos e valores em Remunerações. Cada ajuste mantém a origem e invalida o resultado anterior.</Notice>
  </section>;
}
