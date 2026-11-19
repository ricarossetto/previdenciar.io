import {render,screen,waitFor,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {describe,it,expect,vi,beforeEach} from 'vitest';
import App from '../src/App';
import {RobotAssistant} from '../src/components/RobotAssistant';
import {blankCase,blankInterview,blankWorkspace} from '../src/model';

const invoke=vi.hoisted(()=>vi.fn());
vi.mock('@tauri-apps/api/core',()=>({invoke}));

beforeEach(()=>{
  const workspace=blankWorkspace();
  workspace.calculations=[{...blankCase(),id:1,name:'Entrevista em aberto',assistant:{...blankInterview(),current_benefit_received:false,import_requested:true},identity:{...blankCase().identity,full_name:'Pessoa fictícia',birth_date:'1960-01-01'},sex:'Feminino',disability:'Sem deficiência declarada'}];
  invoke.mockReset();
  invoke.mockImplementation(async(cmd:string)=>cmd==='bootstrap'?{workspace,storage_ready:true,message:null}:cmd==='save_workspace'?workspace:null);
});

describe('Zeigarnik resume loop',()=>{
  it('surfaces one started case with its exact next step and resumes it',async()=>{
    const user=userEvent.setup();render(<App/>);
    await screen.findByRole('heading',{name:'Início'});
    const panel=screen.getByText('Retomar revisão').closest('section');
    expect(panel).not.toBeNull();
    expect(within(panel!).getByText('Entrevista em aberto')).toBeInTheDocument();
    expect(within(panel!).getByText(/Situação e origem/)).toBeInTheDocument();
    await user.click(within(panel!).getByRole('button',{name:/Retomar de onde parei/}));
    expect(await screen.findByRole('heading',{name:/O segurado recebe algum benefício/})).toBeInTheDocument();
  });
});

describe('Zeigarnik autosave loop',()=>{
  it('autosaves only after an existing draft changes, without an initial write',async()=>{
    const draft={...blankCase(),id:1,name:'Rascunho existente',assistant:{...blankInterview(),mode:'assisted'}};
    const onSave=vi.fn().mockResolvedValue(undefined);const setDraft=vi.fn();
    const view=render(<RobotAssistant draft={draft} setDraft={setDraft} workspace={blankWorkspace()} onSave={onSave} onNotify={vi.fn()} onManual={vi.fn()}/>);
    await new Promise(resolve=>setTimeout(resolve,850));
    expect(onSave).not.toHaveBeenCalled();
    view.rerender(<RobotAssistant draft={{...draft,name:'Rascunho existente alterado'}} setDraft={setDraft} workspace={blankWorkspace()} onSave={onSave} onNotify={vi.fn()} onManual={vi.fn()}/>);
    await waitFor(()=>expect(onSave).toHaveBeenCalledWith(expect.objectContaining({name:'Rascunho existente alterado'}),{quiet:true}),{timeout:2000});
    view.unmount();
  });
});
