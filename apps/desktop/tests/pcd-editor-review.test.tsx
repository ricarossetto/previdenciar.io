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
