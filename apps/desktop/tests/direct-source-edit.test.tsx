import {render,screen,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {useState} from 'react';
import {describe,it,expect,vi} from 'vitest';
import {CaseWorkbench} from '../src/components/CaseWorkbench';
import {blankCase,blankWorkspace} from '../src/model';
import type {Calculation,Cnis} from '../src/model';
import {sourceLinkDraft,saveLinkAdjustment} from '../src/links';
function sample(){const c=blankCase();c.name='Hipótese fictícia';c.cnis={metadata:{page_count:1,source_sha256:'d'.repeat(64),extractor:'synthetic'},identity:c.identity,links:[{sequence:1,raw_employer:'EMPREGADOR FICTÍCIO',start_date:'1982-01-16',end_date:null,prpps:true,indicators:['PRPPS'],reported_last_salary_month:'1982-07',inferred_last_salary_end:{date_candidate:'1982-07-31',requires_confirmation:true},source:{page:1}}],salary_entries:[1,2].map(index=>({competence:'1982-06',amount_centavos:index===1?10010:20020,indicators:['PRPPS'],link_sequence:1,source:{page:1,line:index,index},excluded_from_validated_domain:true})),salary_months:{'1982-06':{count:2,amount_sum_centavos:30030,contains_unresolved_prpps:true}},benefits:[],warnings:[],findings:[]} as Cnis;return c;}
function Harness({initial,step,onChange,notify}:{initial:Calculation;step:number;onChange:(c:Calculation)=>void;notify:(s:string,e?:boolean)=>void}){const[c,set]=useState(initial);return <CaseWorkbench draft={c} setDraft={v=>{set(v);onChange(v)}} workspace={blankWorkspace()} onSave={async()=>{}} onNotify={notify} initialStep={step}/>}
describe('direct source editing',()=>{
  it('clicking a source date edits the same link without confirming candidate ends',async()=>{
    const c=sample(),origin=JSON.stringify(c.cnis),change=vi.fn(),notify=vi.fn(),u=userEvent.setup();render(<Harness initial={c} step={3} onChange={change} notify={notify}/>);
    await u.click(screen.getByRole('button',{name:'Editar fim do vínculo 1'}));const d=screen.getByRole('dialog',{name:'Editar vínculo 1'});expect(within(d).getByLabelText('Fim declarado')).toHaveValue('');expect(within(d).getByText(/Data candidata/)).toHaveTextContent('31/07/1982');
    await u.clear(within(d).getByLabelText('Empregador'));await u.type(within(d).getByLabelText('Empregador'),'EMPREGADOR FICTÍCIO RETIFICADO');await u.type(within(d).getByLabelText('Justificativa documental do vínculo'),'Documento fictício de retificação');await u.click(within(d).getByRole('button',{name:'Salvar ajuste do vínculo'}));
    const next=change.mock.calls.at(-1)![0] as Calculation;expect(JSON.stringify(next.cnis)).toBe(origin);expect(next.manual_links[0].employer).toContain('RETIFICADO');expect(next.manual_links[0].end_date).toBe('');expect(next.manual_links[0].source_review!.history).toHaveLength(1);expect(next.manual_links[0].confirmed).toBe(false);expect(next.result).toBeNull();
    await u.click(screen.getByRole('button',{name:'Editar vínculo 1',exact:true}));await u.click(screen.getByRole('button',{name:'Desfazer último ajuste do vínculo'}));const restored=change.mock.calls.at(-1)![0] as Calculation;expect(restored.manual_links[0].employer).toBe('EMPREGADOR FICTÍCIO');expect(restored.manual_links[0]).not.toHaveProperty('excluded');expect(restored.manual_links[0].source_review!.history).toHaveLength(2);
  });
  it('clicking a historical amount selects its individual entries without summing them into one edit',async()=>{
    const c=sample(),origin=JSON.stringify(c.cnis),change=vi.fn(),notify=vi.fn(),u=userEvent.setup();render(<Harness initial={c} step={4} onChange={change} notify={notify}/>);
    await u.click(screen.getByRole('button',{name:'Editar remunerações de 1982-06'}));const chooser=screen.getByRole('dialog',{name:'Editar competência 1982-06'});expect(within(chooser).getByRole('button',{name:'Editar lançamento source:1 da competência 1982-06'})).toHaveTextContent('100,10');expect(within(chooser).getByRole('button',{name:'Editar lançamento source:2 da competência 1982-06'})).toHaveTextContent('200,20');
    await u.click(within(chooser).getByRole('button',{name:'Editar lançamento source:1 da competência 1982-06'}));const d=screen.getByRole('dialog',{name:'Editar contribuição'});await u.clear(within(d).getByLabelText('Valor nominal ajustado'));await u.type(within(d).getByLabelText('Valor nominal ajustado'),'101,11');await u.type(within(d).getByLabelText('Justificativa documental do ajuste'),'Retificação histórica fictícia');await u.click(within(d).getByRole('button',{name:'Salvar ajuste'}));
    expect(screen.getByRole('heading',{name:'Competências e remunerações'})).toBeInTheDocument();const next=change.mock.calls.at(-1)![0] as Calculation;expect(next.contribution_review!.records.map(r=>r.amount)).toEqual(['101.11','200.20']);expect(next.contribution_review!.records[0].currency).toBe('unconfirmed');expect(JSON.stringify(next.cnis)).toBe(origin);
    await u.click(screen.getByRole('button',{name:'Editar competência 1982-06'}));expect(screen.getByRole('dialog',{name:'Editar competência 1982-06'})).toBeInTheDocument();
  });
  it('documentary source edits require evidence and invalid dates do not change the case',()=>{
    const c=sample(),l=sourceLinkDraft(c,c.cnis!.links[0]);expect(()=>saveLinkAdjustment(c,{...l,end_date:'2020-02-30'},null,'Documento fictício')).toThrow(/datas/);expect(()=>saveLinkAdjustment(c,l,null,'')).toThrow(/justificativa/);expect(c.manual_links).toHaveLength(0);
    const next=saveLinkAdjustment(c,{...l,end_date:'1989-04-04'},null,'Documento fictício');expect(next.manual_links[0].end_date).toBe('1989-04-04');expect(c.cnis!.links[0].end_date).toBeNull();
  });
});

it('exact unconfirmed legacy source preparation is linked without duplicating the period',()=>{const c=sample();c.manual_links=[{id:42,employer:'EMPREGADOR FICTÍCIO',start_date:'16/01/1982',end_date:'',regime:'RPPS',indicators:'PRPPS',confirmed:false}];const draft=sourceLinkDraft(c,c.cnis!.links[0]);expect(draft.id).toBe(42);const saved=saveLinkAdjustment(c,{...draft,end_date:'1989-04-04'},null,'Documento fictício');expect(saved.manual_links).toHaveLength(1);expect(saved.manual_links[0].id).toBe(42);expect(saved.manual_links[0].source_review!.history).toHaveLength(1)});
