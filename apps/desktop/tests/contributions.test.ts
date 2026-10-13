import { describe, expect, it } from 'vitest';
import { blankCase, blankInterview } from '../src/model';
import type { Calculation, Cnis } from '../src/model';
import { applyContributionReview, beginContributionReview, canonicalAmount, changeContributions, mergeContributions, undoContributionChange, fromCents, cents, amountLabel } from '../src/contributions';
function imported(): Calculation {
  const c = blankCase();
  c.cnis = { metadata: { page_count:1, source_sha256:'a'.repeat(64), extractor:'synthetic' }, identity:c.identity,
    salary_entries: [1,2].map(index => ({ competence:'2020-01',amount_centavos:index === 1 ? 10010 : 20020,indicators:[],link_sequence:null,source:{page:1,line:index,index},excluded_from_validated_domain:false })),
    salary_months: {'2020-01':{count:2,amount_sum_centavos:30030,contains_unresolved_prpps:false}}, links:[],benefits:[],warnings:[],findings:[] } as Cnis;
  c.result = {status:'experimental',data:{synthetic:true}};c.assistant={...blankInterview(),review_confirmed:true,completed_steps:['review','result']};
  return c;
}
function confirm(c:Calculation):Calculation {
  return changeContributions(c,c.contribution_review!.records.map(r=>({...r,currency:'BRL',confirmed:true,evidence:'Documento fictício conferido'})),'confirm','Documento fictício conferido');
}
describe('contribution review',()=>{
  it('keeps exact cents at and above JS integer precision',()=>{
    expect(canonicalAmount('1.234,56')).toBe('1234.56');
    expect(canonicalAmount('12.50')).toBe('12.50');expect(canonicalAmount('12.5')).toBe('12.50');
    expect(amountLabel('90071992547409.93')).toBe('90.071.992.547.409,93');
    expect(fromCents(cents('92233720368547758.07'))).toBe('92233720368547758.07');
    for(const invalid of ['-1,00','1,001','1e3','NaN','92233720368547758.08'])expect(()=>canonicalAmount(invalid)).toThrow();
  });
  it('prepares all source entries separately and never confirms imported values',()=>{
    const c=imported(), before=JSON.stringify(c.cnis);const next=beginContributionReview(c);
    expect(next.contribution_review!.records).toHaveLength(2);
    expect(next.contribution_review!.records.map(r=>r.amount)).toEqual(['100.10','200.20']);
    expect(next.contribution_review!.records.every(r=>!r.confirmed&&r.currency==='unconfirmed')).toBe(true);
    expect(JSON.stringify(next.cnis)).toBe(before);expect(next.result).toBeNull();expect(next.assistant!.review_confirmed).toBe(false);
  });
  it('rejects unsafe imported numbers atomically',()=>{
    const c=imported();c.cnis!.salary_entries[1].amount_centavos=Number.MAX_SAFE_INTEGER+1;
    expect(()=>beginContributionReview(c)).toThrow(/precisão/);expect(c.contribution_review).toBeNull();
  });
  it('merges documented concomitance with exact cents and invalidates confirmation',()=>{
    const c=confirm(beginContributionReview(imported()));
    expect(()=>applyContributionReview(c)).toThrow(/Concomitância/);
    const merged=mergeContributions(c,c.contribution_review!.records.map(r=>r.key),'Consolidação sintética documentada');
    expect(merged.contribution_review!.records[0].amount).toBe('300.30');
    expect(merged.contribution_review!.records[0].source_indices).toEqual([1,2]);
    expect(merged.contribution_review!.records[0].confirmed).toBe(false);
    const applied=applyContributionReview(confirm(merged));expect(applied.manual_salaries[0].amount).toBe('300.30');expect(applied.contribution_review!.applied).toBe(true);
  });
  it('excludes without deleting source and documents multiple undo operations',()=>{
    let c=beginContributionReview(imported());const source=JSON.stringify(c.cnis);
    c=changeContributions(c,c.contribution_review!.records.map((r,i)=>i? r:{...r,excluded:true,evidence:'Não usar nesta hipótese'}),'exclude','Não usar nesta hipótese');
    c=changeContributions(c,c.contribution_review!.records.map((r,i)=>i? {...r,amount:'200.21',evidence:'Retificação fictícia'}:r),'edit','Retificação fictícia');
    c=undoContributionChange(c);expect(c.contribution_review!.records.find(r=>r.key==='source:2')!.amount).toBe('200.20');
    c=undoContributionChange(c);expect(c.contribution_review!.records.every(r=>!r.excluded)).toBe(true);expect(JSON.stringify(c.cnis)).toBe(source);
    expect(c.contribution_review!.history.filter(r=>r.action==='undo')).toHaveLength(2);
  });
  it('blocks unapplied edits, mixed currencies, indicators and pre-PBC months',()=>{
    let c=beginContributionReview(imported());expect(()=>applyContributionReview(c)).toThrow(/conferência/);
    let r=c.contribution_review!.records[0];
    c=changeContributions(c,[{...r,currency:'BRL',confirmed:true,evidence:'Conferido'}, {...c.contribution_review!.records[1],excluded:true,evidence:'Exclusão justificada'}],'confirm','Revisão sintética');
    for(const change of [{currency:'URV'},{indicators:'PRPPS'},{competence:'1994-06'}]){
      const changed=changeContributions(c,[{...c.contribution_review!.records[0],...change},c.contribution_review!.records[1]],'edit','Hipótese fictícia alterada');expect(()=>applyContributionReview(changed)).toThrow();
    }
    const applied=applyContributionReview(c);const changed=changeContributions(applied,applied.contribution_review!.records.map((r,i)=>i?r:{...r,amount:'101.00',evidence:'Retificação'}),'edit','Retificação');
    expect(changed.contribution_review!.applied).toBe(false);expect(changed.result).toBeNull();expect(changed.manual_salaries[0].amount).toBe('100.10');
  });
  it('can undo an operation whose documentary reason reaches the maximum length',()=>{
    let c=beginContributionReview(imported());const reason='x'.repeat(1000);
    c=changeContributions(c,c.contribution_review!.records.map((r,i)=>i?r:{...r,amount:'100.11',evidence:reason}),'edit',reason);
    const undo=undoContributionChange(c);expect(undo.contribution_review!.records.find(r=>r.key==='source:1')!.amount).toBe('100.10');
  });
  it('requires documentary evidence and caps audit growth without dropping operations',()=>{
    const c=beginContributionReview(imported());const rows=c.contribution_review!.records.map(r=>({...r,excluded:true,evidence:''}));
    expect(()=>changeContributions(c,rows,'exclude','')).toThrow();
    expect(()=>changeContributions(c,rows,'exclude','Justificativa')).toThrow(/justificativa/);
    expect(c.contribution_review!.history).toHaveLength(0);
  });
});
