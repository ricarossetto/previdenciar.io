import {useState} from 'react';
import {render,screen,act} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {expect,it,vi} from 'vitest';
import {CaseWorkbench} from '../src/components/CaseWorkbench';
import {api} from '../src/api';
import {blankCase,blankWorkspace} from '../src/model';
import type {Json} from '../src/model';
it('a late engine response cannot overwrite edits made during calculation',async()=>{
  let resolve!:(value:Json)=>void;const pending=new Promise<Json>(done=>{resolve=done});const call=vi.spyOn(api,'calculate').mockReturnValueOnce(pending);const notify=vi.fn();let latest=blankCase();
  function Harness(){const[draft,setDraft]=useState({...blankCase(),name:'Hipótese fictícia inicial'});latest=draft;return <CaseWorkbench draft={draft} setDraft={setDraft} workspace={blankWorkspace()} onSave={async()=>{}} onNotify={notify} initialStep={7}/>}
  const u=userEvent.setup();render(<Harness/>);await u.click(screen.getByRole('button',{name:'Executar motor RGPS'}));await u.click(screen.getByRole('button',{name:'1. Identificação'}));const input=screen.getByLabelText('Nome do cálculo');await u.clear(input);await u.type(input,'Hipótese fictícia alterada');
  await act(async()=>resolve({status:'experimental',data:{synthetic:true},warnings:[]}));expect(latest.name).toBe('Hipótese fictícia alterada');expect(latest.result).toBeNull();expect(notify).toHaveBeenCalledWith(expect.stringContaining('resultado antigo descartado'),true);call.mockRestore();
});
