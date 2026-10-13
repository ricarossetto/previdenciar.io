import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ContributionEditor } from '../src/components/ContributionEditor';
import { blankCase } from '../src/model';
import type { Calculation, Cnis } from '../src/model';
function seed() {
  const c=blankCase();c.name='Caso fictício do livro';
  c.cnis={metadata:{page_count:1,source_sha256:'b'.repeat(64),extractor:'synthetic'},identity:c.identity,
    salary_entries:[1,2].map(index=>({competence:'2020-01',amount_centavos:index===1?10010:20020,indicators:[],link_sequence:null,source:{page:1,line:index,index},excluded_from_validated_domain:false})),
    salary_months:{'2020-01':{count:2,amount_sum_centavos:30030,contains_unresolved_prpps:false}},links:[],benefits:[],warnings:[],findings:[]} as Cnis;return c;
}
function Harness({initial,onUpdate,onNotify}:{initial:Calculation;onUpdate:(c:Calculation)=>void;onNotify:(s:string,error?:boolean)=>void}){
  const [draft,setDraft]=useState(initial);return <ContributionEditor draft={draft} setDraft={c=>{setDraft(c);onUpdate(c)}} onNotify={onNotify}/>;
}
describe('contribution editor',()=>{
  it('edits an imported value with documentary evidence and keeps source intact',async()=>{
    const initial=seed(), source=JSON.stringify(initial.cnis), updates=vi.fn(), notify=vi.fn(),user=userEvent.setup();
    render(<Harness initial={initial} onUpdate={updates} onNotify={notify}/>);
    await user.click(screen.getByRole('button',{name:'Abrir livro de contribuições'}));
    await user.click(screen.getByRole('button',{name:'Editar contribuição source:1'}));
    const d=screen.getByRole('dialog');await user.clear(within(d).getByLabelText('Valor nominal ajustado'));await user.type(within(d).getByLabelText('Valor nominal ajustado'),'200,25');
    await user.type(within(d).getByLabelText('Justificativa documental do ajuste'),'Retificação fictícia documentada');await user.selectOptions(within(d).getByLabelText('Moeda declarada'),'BRL');
    await user.click(within(d).getByRole('button',{name:'Salvar ajuste'}));expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    const next=updates.mock.calls.at(-1)![0] as Calculation;expect(next.contribution_review!.records[0].amount).toBe('200.25');expect(JSON.stringify(next.cnis)).toBe(source);expect(next.contribution_review!.applied).toBe(false);expect(next.result).toBeNull();
    await user.click(screen.getByRole('button',{name:'Desfazer última alteração'}));expect((updates.mock.calls.at(-1)![0] as Calculation).contribution_review!.records.find(r=>r.key==='source:1')!.amount).toBe('100.10');
  });
  it('applies an explicitly consolidated and confirmed month, not just the displayed total',async()=>{
    const updates=vi.fn(),notify=vi.fn(),user=userEvent.setup();render(<Harness initial={seed()} onUpdate={updates} onNotify={notify}/>);
    await user.click(screen.getByRole('button',{name:'Abrir livro de contribuições'}));
    await user.type(screen.getByLabelText('Justificativa documental da ação em lote'),'Documento fictício conferido');
    await user.click(screen.getByLabelText('Selecionar lançamentos desta página'));await user.click(screen.getByRole('button',{name:'Declarar BRL nos selecionados'}));
    await user.click(screen.getByLabelText('Selecionar lançamentos desta página'));await user.click(screen.getByRole('button',{name:'Consolidar competência selecionada'}));
    await user.click(screen.getByLabelText('Selecionar lançamentos desta página'));await user.click(screen.getByRole('button',{name:'Conferir selecionados'}));
    await user.click(screen.getByRole('button',{name:'Aplicar revisão ao cálculo'}));
    const next=updates.mock.calls.at(-1)![0] as Calculation;expect(next.manual_salaries).toHaveLength(1);expect(next.manual_salaries[0].amount).toBe('300.30');expect(next.contribution_review!.applied).toBe(true);expect(screen.getByText('Aplicado aos salários')).toBeInTheDocument();
  });
  it('requires evidence and never applies merely selected or unconfirmed rows',async()=>{
    const updates=vi.fn(),notify=vi.fn(),user=userEvent.setup();render(<Harness initial={seed()} onUpdate={updates} onNotify={notify}/>);
    await user.click(screen.getByRole('button',{name:'Abrir livro de contribuições'}));await user.click(screen.getByRole('button',{name:'Aplicar revisão ao cálculo'}));
    expect(notify.mock.calls.at(-1)![1]).toBe(true);expect((updates.mock.calls.at(-1)![0] as Calculation).manual_salaries).toHaveLength(0);
    await user.click(screen.getByLabelText('Selecionar lançamentos desta página'));await user.click(screen.getByRole('button',{name:'Excluir selecionados da hipótese'}));expect(notify.mock.calls.at(-1)![1]).toBe(true);
    expect((updates.mock.calls.at(-1)![0] as Calculation).contribution_review!.records.every(r=>!r.excluded)).toBe(true);
  });
});
