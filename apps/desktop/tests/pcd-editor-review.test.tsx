import {useState} from 'react';
import {render,screen} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {expect,it,vi} from 'vitest';
import {CaseWorkbench} from '../src/components/CaseWorkbench';
import {api} from '../src/api';
import {blankCase,blankInterview,blankWorkspace} from '../src/model';

it('PcD editor lets the operator confirm the assistant review after changing the policy',async()=>{
  const base={...blankCase(),name:'PcD fictício',disability:'Grave',assistant:{...blankInterview(),mode:'assisted',current_benefit_received:false,import_requested:true,process_kind:'Simulação do benefício',goal:'Calcular um novo benefício',review_confirmed:false,completed_steps:['intro','benefit','import','process','goal','parameters','periods','salaries','policy']}};
  const call=vi.spyOn(api,'calculate').mockResolvedValue({status:'blocked',blockers:[]});
  function Harness(){const[draft,setDraft]=useState(base);return <CaseWorkbench draft={draft} setDraft={setDraft} workspace={blankWorkspace()} onSave={async()=>{}} onNotify={()=>{}} initialStep={7}/>}
  const u=userEvent.setup();
  render(<Harness/>);
  expect(screen.getByRole('button',{name:'Executar motor RGPS'})).toBeDisabled();
  expect(screen.getByText(/Confirme a revisão na etapa/)).toBeInTheDocument();
  await u.click(screen.getByRole('button',{name:'Ir para revisão'}));
  await u.click(screen.getByRole('checkbox',{name:/Confirmo a revisão/}));
  await u.click(screen.getByRole('button',{name:'Ir para resultado'}));
  const execute=screen.getByRole('button',{name:'Executar motor RGPS'});
  expect(execute).toBeEnabled();
  await u.click(execute);
  expect(call).toHaveBeenCalledTimes(1);
  call.mockRestore();
});

it('PcD editor points to the contribution book when manual salaries are not applied',()=>{
  const draft={...blankCase(),name:'PcD fictício',disability:'Grave',manual_salaries:[{id:1,competence:'2020-01',amount:'1000.00',indicators:'',confirmed:true}],contribution_review:null};
  render(<CaseWorkbench draft={draft} setDraft={()=>{}} workspace={blankWorkspace()} onSave={async()=>{}} onNotify={()=>{}} initialStep={5}/>);
  expect(screen.getByText(/O PcD usa o livro de contribuições revisado/)).toBeInTheDocument();
  expect(screen.getByRole('button',{name:'Abrir o livro de contribuições'})).toBeInTheDocument();
});

it('PcD editor can drop manual salaries and use the CNIS source',async()=>{
  const base={...blankCase(),name:'PcD fonte',disability:'Grave',manual_salaries:[{id:1,competence:'2020-01',amount:'1000.00',indicators:'',confirmed:true}],contribution_review:null,assistant:{...blankInterview(),mode:'assisted',current_benefit_received:false,import_requested:true,process_kind:'Simulação do benefício',goal:'Calcular um novo benefício',review_confirmed:true,completed_steps:[]}};
  function Harness(){const[draft,setDraft]=useState(base);return <CaseWorkbench draft={draft} setDraft={setDraft} workspace={blankWorkspace()} onSave={async()=>{}} onNotify={()=>{}} initialStep={5}/>}
  const u=userEvent.setup();
  render(<Harness/>);
  await u.click(screen.getByRole('button',{name:'Usar os salários do CNIS no PcD'}));
  expect(screen.queryByText(/O PcD usa o livro/)).not.toBeInTheDocument();
  expect(screen.getByText(/Simulação PcD pela LC 142/)).toBeInTheDocument();
});
