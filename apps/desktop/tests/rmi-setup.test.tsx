import {render,screen} from '@testing-library/react';
import {it,expect,vi} from 'vitest';
import {CaseWorkbench} from '../src/components/CaseWorkbench';
import {blankCase,blankWorkspace} from '../src/model';
it('RMI setup does not expose four tabs that merely change a highlight',()=>{
 const draft=blankCase(),setDraft=vi.fn();render(<CaseWorkbench draft={draft} setDraft={setDraft} workspace={blankWorkspace()} onSave={async()=>{}} onNotify={()=>{}} initialStep={5}/>);
 expect(screen.getByRole('heading',{name:'Opções da RMI',exact:true})).toBeInTheDocument();
 for(const name of ['EC 103 / transições','Lei 9.876 / fator','Direito adquirido','Juros e honorários'])expect(screen.queryByRole('button',{name,exact:true})).not.toBeInTheDocument();
 expect(screen.queryByLabelText('Juros por mês (%)')).not.toBeInTheDocument();expect(setDraft).not.toHaveBeenCalled();
});
it('a disability scenario exposes documented constant-grade assumptions rather than a false legal rule selector',()=>{
 render(<CaseWorkbench draft={{...blankCase(),disability:'Grave'}} setDraft={()=>{}} workspace={blankWorkspace()} onSave={async()=>{}} onNotify={()=>{}} initialStep={5}/>);
 expect(screen.getByRole('heading',{name:/Simulação PcD pela LC 142/})).toBeInTheDocument();expect(screen.getByLabelText(/Conferi e assumo explicitamente/)).not.toBeChecked();expect(screen.getByRole('button',{name:'Consultar índices e limites públicos'})).toBeDisabled();
});
