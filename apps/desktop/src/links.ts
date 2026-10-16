import type {Calculation,CnisLink,ManualLink,LinkSnapshot} from './model';
import {isoDate,prepareImportedLinksForReview,invalidateRmiAssumptions} from './model';
export function linkSnapshot(link:ManualLink):LinkSnapshot{return{employer:link.employer,start_date:isoDate(link.start_date)||link.start_date,end_date:isoDate(link.end_date)||link.end_date,regime:link.regime,indicators:link.indicators,confirmed:link.confirmed,excluded:link.source_review?.excluded??false}}
export function sourceLinkDraft(c:Calculation,source:CnisLink):ManualLink{
  return prepareImportedLinksForReview(c).find(l=>l.source_review?.sequence===source.sequence)!;
}
export function saveLinkAdjustment(c:Calculation,next:ManualLink,index:number|null,reason:string):Calculation{
  if(next.start_date&&!isoDate(next.start_date)||next.end_date&&!isoDate(next.end_date))throw Error('Informe datas válidas ou mantenha a data ausente.');
  if(next.start_date&&next.end_date&&isoDate(next.start_date)!>isoDate(next.end_date)!)throw Error('O fim não pode ser anterior ao início.');
  if(next.employer.length>1000||next.indicators.length>1000)throw Error('Texto do vínculo excede 1.000 caracteres.');
  let value={...next,start_date:isoDate(next.start_date)||'',end_date:isoDate(next.end_date)||''};
  let rows=[...c.manual_links];
  if(next.source_review){
    if(!reason.trim()||reason.length>1000)throw Error('Informe justificativa documental de até 1.000 caracteres.');
    const source=c.cnis?.links.find(l=>l.sequence===next.source_review!.sequence);
    if(!source||next.source_review.source_sha256!==(c.cnis?.metadata.source_sha256??null))throw Error('Origem do vínculo diverge do CNIS; preserve a hipótese.');
    index=rows.findIndex(l=>l.source_review?.sequence===source.sequence);
    if(index<0){const base=sourceLinkDraft(c,source);index=rows.findIndex(l=>!l.source_review&&!l.confirmed&&JSON.stringify(linkSnapshot(l))===JSON.stringify(linkSnapshot(base)));}
    if(index<0)index=null;
    const previous=index===null||!rows[index].source_review?sourceLinkDraft(c,source):rows[index];
    const history=previous.source_review!.history;
    if(history.length>=200)throw Error('Histórico do vínculo atingiu o limite.');
    const after=linkSnapshot(value),before=linkSnapshot(previous);
    value={...value,source_review:{...next.source_review,history:[...history,{at:new Date().toISOString(),reason:reason.trim(),before,after}]}};
  }
  if(index===null)rows.push(value);else rows=rows.map((row,i)=>i===index?value:row);
  return {...invalidateRmiAssumptions(c),manual_links:rows,result:null,status:'Revisão pendente',assistant:c.assistant?{...c.assistant,review_confirmed:false,completed_steps:c.assistant.completed_steps.filter(s=>!['review','result'].includes(s))}:null};
}
