import{CnisReview}from'./CnisReview';import{AssistantConsiderationPanel}from'./AssistantConsideration';import'../styles/assistant.css';
import{useState,useRef,useEffect}from'react';
import type{AssistantInterview,Calculation,Cnis,Workspace,Json}from'../model';
import{invalidateRmiAssumptions,interviewOf,withInterviewAnswer,robotParameterIssues,brDate,isoDate,hasFinancialResult}from'../model';
import{assistantConsideration}from'../assistantReview';
import type{ConsiderationFix}from'../assistantReview';
import{api,errorMessage}from'../api';
import{Badge,Button,Field,Icon,Notice,Panel,Select}from'./ui';
import{CaseWorkbench,ResultView}from'./CaseWorkbench';
import{PrepareFromCnis}from'./PrepareFromCnis';
import{ReportCenter}from'./ReportCenter';

type Screen='intro'|'situation'|'objective'|'parameters'|'conference'|'review'|'result';
const SCREENS:Screen[]=['intro','situation','objective','parameters','conference','review','result'];
const LABEL:Record<Screen,string>={
  intro:'Boas-vindas',
  situation:'Situação e origem',
  objective:'Objetivo do cálculo',
  parameters:'Segurado e DIB',
  conference:'Conferência dos dados',
  review:'Revisar e executar',
  result:'Resultado e relatórios',
};
type ConferenceTab='periods'|'salaries'|'policy';
const TABS:{id:ConferenceTab;label:string;detail:string;step:number}[]=[
  {id:'periods',label:'Conferir períodos',detail:'Vínculos, regime, datas e exclusões',step:3},
  {id:'salaries',label:'Conferir remunerações',detail:'Competências, valores e revisão',step:4},
  {id:'policy',label:'Conferir opções da RMI',detail:'Correção, descarte e limites',step:5},
];

export function assistantStepState(c:Calculation,screen:Screen):'confirmed'|'visited'|'pending'{
  const a=interviewOf(c);
  if(screen==='intro')return c.name.trim()?'confirmed':'pending';
  if(screen==='situation')return a.current_benefit_received!==null&&a.import_requested!==null&&(!a.import_requested||Boolean(c.cnis))?'confirmed':a.completed_steps.includes('benefit')?'visited':'pending';
  if(screen==='objective')return a.process_kind&&a.goal?'confirmed':'pending';
  if(screen==='parameters')return robotParameterIssues(c).length===0?'confirmed':'pending';
  if(screen==='conference')return assistantConsideration(c).ready?'confirmed':(['periods','salaries','policy'].some(item=>a.completed_steps.includes(item))?'visited':'pending');
  if(screen==='review')return a.review_confirmed?'confirmed':'pending';
  return hasFinancialResult(c)?'confirmed':c.result?'visited':'pending';
}

function initialScreen(c:Calculation):Screen{
  if(c.result)return'result';
  const a=interviewOf(c);
  if(!c.name.trim())return'intro';
  if(a.current_benefit_received===null||a.import_requested===null||(a.import_requested&&!c.cnis))return'situation';
  if(!a.process_kind||!a.goal)return'objective';
  if(robotParameterIssues(c).length)return'parameters';
  if(!a.completed_steps.includes('policy'))return'conference';
  if(!a.review_confirmed)return'review';
  return'result';
}
function withSteps(c:Calculation,answer:Partial<AssistantInterview>,steps:string[]):Calculation{
  let next=withInterviewAnswer(c,answer);
  for(const step of steps)next=withInterviewAnswer(next,{},step);
  return next;
}
function markStep(c:Calculation,step:string):Calculation{
  if(!c.assistant||c.assistant.mode!=='assisted'||c.assistant.completed_steps.includes(step))return c;
  return {...c,assistant:{...c.assistant,completed_steps:[...c.assistant.completed_steps,step]}};
}
const PROCESS_TITLES:Record<string,string>={
  'Simulação do benefício':'Simular um novo benefício',
  'Revisão e valor da causa':'Calcular revisão e valor da causa',
  'Planejamento futuro':'Planejar uma aposentadoria futura',
  'Processo administrativo':'Analisar um processo administrativo no INSS',
  'Execução judicial':'Calcular execução judicial',
  'Contribuições em atraso':'Calcular contribuições em atraso',
};
// Três escolhas na primeira dobra; o resto fica a um clique, para não abrir
// seis cartões grandes de uma vez na frente de quem usa pouco o computador.
const PRIMARY_PROCESS=['Simulação do benefício','Processo administrativo','Revisão e valor da causa'];
const OTHER_PROCESS=['Planejamento futuro','Execução judicial','Contribuições em atraso'];
function choiceLabel(process:string){
  return process==='Simulação do benefício'?'Organizar os dados para uma hipótese, sem afirmar concessão.':process==='Revisão e valor da causa'?'Registrar a intenção; a modalidade depende de motor e dados próprios.':process==='Planejamento futuro'?'Registrar objetivo, sem inventar salários ou contribuições futuras.':process==='Processo administrativo'?'Resposta declarada pelo operador, não extraída de um indeferimento antigo.':process==='Execução judicial'?'A execução integral não é presumida como suportada pelo motor RGPS.':'Modalidade separada, com parâmetros e regras específicas.';
}

/// Mostra na conferência exatamente o que a entrevista respondeu. Sem isto, uma
/// resposta perdida vira uma pendência sem origem visível e o operador precisa
/// adivinhar em qual etapa ela nasceu.
function ConferenceInterview({draft,onFix}:{draft:Calculation;onFix:(screen:Screen)=>void}){
  const a=interviewOf(draft);
  const answered=a.current_benefit_received!==null;
  const benefit=answered?(a.current_benefit_received?'Sim, recebe benefício (declarado)':'Não recebe benefício (declarado)'):'Não confirmado';
  const rows:[string,string][]=[
    ['Benefício atual',benefit],
    ['Finalidade',a.process_kind??'Não informada'],
    ['Objetivo',a.goal??'Não informado'],
    ['Segurado',(draft.identity.full_name??'Não informado')+(draft.identity.birth_date?' · nascimento '+brDate(draft.identity.birth_date):'')],
    ['Sexo e deficiência',(draft.sex??'Pendente')+' · '+(draft.disability??'Pendente')],
    ['DIB',draft.dib||'Não informada'],
  ];
  return <section className="conference-interview" aria-label="Respostas da entrevista">
    <header className="conference-interview__head">
      <div><span className="eyebrow">Antes de calcular, 1 de 2</span><h3>Confirme as respostas da entrevista</h3></div>
      <Badge tone={answered?'good':'review'}>{answered?'Respostas registradas':'Falta a resposta de benefício'}</Badge>
    </header>
    <dl className="conference-interview__answers">{rows.map(([label,value])=><div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}</dl>
    {!answered&&<Notice>Responda na etapa de situação e origem se o segurado recebe benefício hoje. O programa não infere essa resposta a partir do CNIS nem do nome.</Notice>}
    <div className="form-actions" data-ui-actions>
      <Button onClick={()=>onFix('situation')}>Corrigir benefício e importação</Button>
      <Button onClick={()=>onFix('objective')}>Corrigir finalidade e objetivo</Button>
      <Button onClick={()=>onFix('parameters')}>Corrigir dados do segurado</Button>
    </div>
  </section>;
}

export function RobotFace({large=false}:{large?:boolean}){return <span className={'robot-face '+(large?'large':'')} aria-hidden="true"><svg viewBox="0 0 64 64" fill="none"><path d="M32 7v7" stroke="currentColor" strokeWidth="3"/><circle cx="32" cy="5" r="3" fill="currentColor"/><rect x="12" y="15" width="40" height="34" rx="12" stroke="currentColor" strokeWidth="2.5"/><rect x="6" y="25" width="6" height="14" rx="3" fill="currentColor"/><rect x="52" y="25" width="6" height="14" rx="3" fill="currentColor"/><circle cx="25" cy="29" r="3.5" fill="currentColor"/><circle cx="39" cy="29" r="3.5" fill="currentColor"/><path d="M24 40c5 4 11 4 16 0M22 49v7M42 49v7" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round"/></svg></span>}
function Choice({title,text,onClick,selected=false}:{title:string;text:string;onClick:()=>void;selected?:boolean}){return <button type="button" className={'robot-choice '+(selected?'selected':'')} onClick={onClick}><span><strong>{title}</strong><small>{text}</small></span><Icon name="arrow" size={18}/></button>}

export function RobotAssistant({draft,setDraft,workspace,onSave,onNotify,onManual,onManualAt}:{draft:Calculation;setDraft:(c:Calculation)=>void;workspace:Workspace;onSave:(c:Calculation)=>Promise<void>;onNotify:(s:string,error?:boolean)=>void;onManual:()=>void;onManualAt?:(step:number)=>void}){
  const[screen,setScreen]=useState<Screen>(()=>initialScreen(draft));
  const[tab,setTab]=useState<ConferenceTab>('periods');
  const[busy,setBusy]=useState(false),[pending,setPending]=useState<Cnis|null>(null),[path,setPath]=useState(''),[error,setError]=useState('');
  const generation=useRef(0),latestDraft=useRef(draft);latestDraft.current=draft;
  useEffect(()=>()=>{generation.current+=1},[]);
  const a=interviewOf(draft),issues=robotParameterIssues(draft),consideration=assistantConsideration(draft,a);
  const resultBlockers=(draft.result as Record<string,unknown>|null)?.blockers;
  const pcdBlocked=Array.isArray(resultBlockers)&&resultBlockers.some(item=>String((item as Record<string,unknown>)?.code??'')==='pcd_lc142_not_implemented');
  const current=SCREENS.indexOf(screen);
  const confirmedSteps=SCREENS.filter(item=>assistantStepState(draft,item)==='confirmed').length;
  const conferenceVisited=(['periods','salaries','policy']as ConferenceTab[]).every(item=>a.completed_steps.includes(item));
  useEffect(()=>{
    if(screen!=='conference'||busy)return;
    if(!a.completed_steps.includes(tab))setDraft(markStep(draft,tab));
  },[screen,tab]);
  function update(p:Partial<Calculation>){setDraft({...invalidateRmiAssumptions({...draft,...p}),result:null,assistant:{...interviewOf(draft),review_confirmed:false}})}
  function answer(p:Partial<AssistantInterview>,steps:string[],next?:Screen){setDraft(withSteps(draft,p,steps));setError('');if(next)setScreen(next)}
  function chooseProcess(id:string){const c=withInterviewAnswer({...draft,scenario:id==='Simulação do benefício'?'Simulação de novo benefício':id},{process_kind:id},'process');setDraft(c);setError('')}
  function next(target:Screen,steps:string[]=[]){setDraft(withSteps(draft,{},steps));setError('');setScreen(target)}
  function reachable(target:Screen):boolean{
    if(target==='intro')return true;
    if(target==='situation')return Boolean(draft.name.trim());
    if(target==='objective')return Boolean(draft.name.trim())&&a.current_benefit_received!==null&&a.import_requested!==null;
    if(target==='parameters')return Boolean(a.process_kind&&a.goal);
    if(target==='conference')return Boolean(a.process_kind&&a.goal)&&issues.length===0;
    if(target==='review')return conferenceVisited;
    return a.review_confirmed||Boolean(draft.result);
  }
  async function importCnis(fromPath=false){setBusy(true);setError('');const token=++generation.current;try{const data=fromPath?await api.importPath(path):await api.pickCnis();if(token===generation.current&&data)setPending(data)}catch(e){if(token===generation.current)setError(errorMessage(e))}finally{if(token===generation.current)setBusy(false)}}
  function apply(){
    if(draft.contribution_review||draft.manual_links.some(link=>link.source_review)){setError('Preserve o livro existente: crie outro cálculo para substituir o documento de origem.');return}
    if(!pending)return;
    const value:Calculation={...draft,identity:structuredClone(pending.identity),cnis:pending,sex:null,disability:null,result:null,status:'CNIS importado; entrevista em andamento'};
    setDraft(withInterviewAnswer(value,{import_requested:true},'import'));setPending(null);setScreen('objective');
    onNotify('CNIS aplicado. Não inferimos sexo, deficiência ou data final ausente.');
  }
  async function calculate(){
    if(!a.review_confirmed){setError('Confirme a revisão dos parâmetros antes de executar.');return}
    setBusy(true);setError('');const token=++generation.current;
    try{
      const snapshot=draft,result=await api.calculate(snapshot);
      if(token!==generation.current)return;
      if(latestDraft.current!==snapshot){setError('Hipótese alterada durante a execução; resultado antigo descartado. Revise e execute novamente.');return}
      const c={...draft,result,status:(result as Record<string,Json>)?.status==='blocked'?'Execução bloqueada; revisar dados':'Simulação técnica calculada',assistant:{...a,completed_steps:[...new Set([...a.completed_steps,'result'])]}};
      setDraft(c);
      if((result as Record<string,Json>)?.status==='blocked')onNotify('O motor identificou pendências. Nenhum valor fictício foi gerado.',true);
      else onNotify('Cálculo executado. Confira resultado, memória e avisos.');
    }catch(e){if(token===generation.current)setError(errorMessage(e))}
    finally{if(token===generation.current)setBusy(false)}
  }
  function fix(where:ConsiderationFix){
    if(where==='parameters'){setScreen('parameters');return}
    if(where==='interview'){setScreen(a.current_benefit_received===null?'situation':'objective');return}
    if(where==='salaries'){setTab('salaries');setScreen('conference');return}
    if(where==='source'||where==='policy'){setTab('policy');setScreen('conference');return}
    setTab('periods');setScreen('conference');
  }
  const question:Record<Screen,string>={
    intro:'Olá! Sou seu assistente de cálculos. Vamos analisar esse caso com calma?',
    situation:'O segurado recebe algum benefício previdenciário atualmente?',
    objective:'O que este cálculo precisa resolver?',
    parameters:'Vamos confirmar apenas os dados que alteram o cálculo.',
    conference:'Confira os dados que o motor vai receber.',
    review:'Antes de executar, revise o que está confirmado e o que falta.',
    result:'Execute a hipótese revisada e apresente a memória do motor.',
  };
  const unsupportedProcess=a.process_kind&&!['Simulação do benefício','Processo administrativo'].includes(a.process_kind)?'A finalidade "'+a.process_kind+'" não tem motor próprio neste adaptador. A conferência e os relatórios continuam disponíveis, mas a execução de RMI fica bloqueada.':'';
  const unsupported=a.current_benefit_received===true?'O motor de novo benefício não executa revisão de benefício recebido.':a.goal&&a.goal!=='Calcular um novo benefício'?'Este objetivo pode ser registrado, mas ainda não tem cálculo automático neste assistente.':unsupportedProcess;
  return <div className="robot-workspace">
    {screen!=='intro'&&<><div className="assistant-stepbar"><div><strong>Etapa {current+1} de {SCREENS.length} · {LABEL[screen]}</strong><p>{draft.name||'Novo cálculo RGPS'}</p><progress aria-label="Progresso da entrevista" value={confirmedSteps} max={SCREENS.length}/></div><Badge tone={hasFinancialResult(draft)?'good':'review'}>{draft.result?(hasFinancialResult(draft)?'Resultado disponível':'Pendências do motor'):consideration.ready?'Dados revisados':'Entrevista em andamento'}</Badge></div><details className="assistant-steps"><summary>Ver etapas e revisar respostas</summary><div className="robot-progress" role="group" aria-label="Etapas do assistente">{SCREENS.map((item,index)=><button key={item} type="button" disabled={busy||(index>current&&!reachable(item))} aria-current={screen===item?'step':undefined} className={screen===item?'active':assistantStepState(draft,item)==='confirmed'?'done':''} data-step-state={assistantStepState(draft,item)} onClick={()=>{setScreen(item);setError('')}}>{assistantStepState(draft,item)==='confirmed'?<Icon name="check" size={13}/>:<span>{index+1}</span>}{LABEL[item]}<small>{assistantStepState(draft,item)==='confirmed'?'Confirmada':assistantStepState(draft,item)==='visited'?'Conferência pendente':'Pendente'}</small></button>)}</div></details></>}
    {screen!=='intro'&&<section className="assistant-case-context" aria-label="Resumo do cenário"><div><span>Segurado</span><strong>{draft.identity.full_name||'Identificação pendente'}</strong></div><div><span>Nascimento</span><strong>{draft.identity.birth_date?brDate(draft.identity.birth_date):'Não informado'}</strong></div><div><span>DIB / simulação</span><strong>{draft.dib?brDate(draft.dib):'Não informada'}</strong></div><div><span>Objetivo</span><strong>{a.goal||'A definir'}</strong></div><Button disabled={busy||!reachable('parameters')} onClick={()=>setScreen('parameters')}>Editar dados do segurado</Button></section>}
    {(['conference','review','result']as Screen[]).includes(screen)&&<nav className="assistant-domain-nav" aria-label="Painel do cálculo">
      {TABS.map(item=><button key={item.id} type="button" disabled={busy} aria-current={screen==='conference'&&tab===item.id?'page':undefined} onClick={()=>{setTab(item.id);setScreen('conference');setError('')}}><strong>{item.id==='periods'?'Abrir períodos':item.id==='salaries'?'Abrir remunerações':'Abrir opções da RMI'}</strong><small>{item.id==='periods'?consideration.adjusted.selectedLinks+(consideration.adjusted.selectedLinks===1?' período declarado':' períodos declarados'):item.id==='salaries'?consideration.adjusted.salaries+(consideration.adjusted.salaries===1?' remuneração declarada':' remunerações declaradas'):consideration.issues.filter(issue=>issue.decisive&&['policy','source'].includes(issue.fix)).length+' pendências de opções'}</small></button>)}
      <button type="button" disabled={busy||!conferenceVisited} aria-current={screen==='review'?'page':undefined} onClick={()=>{setScreen('review');setError('')}}><strong>Abrir revisão</strong><small>{consideration.issues.filter(issue=>issue.decisive).length} pendências decisivas</small></button>
      <button type="button" disabled={busy||!reachable('result')} aria-current={screen==='result'?'page':undefined} onClick={()=>{setScreen('result');setError('')}}><strong>Abrir resultado</strong><small>{hasFinancialResult(draft)?'Resultado experimental':draft.result?'Execução bloqueada':'Ainda não executado'}</small></button>
    </nav>}
    <section className="robot-question"><RobotFace/><div><h3>{question[screen]}</h3>{screen==='intro'?<p><strong>Uma etapa de cada vez.</strong> Como numa entrevista curta, vou perguntar apenas o que muda o cálculo e depois abrir a conferência dos dados, sem inventar informação ou direito a benefício.</p>:<p>{draft.name||'Novo cálculo RGPS'} · {LABEL[screen]}</p>}</div></section>
    {error&&<Notice tone="error">{error}</Notice>}
    {unsupported&&<Notice><div className="assistant-scope-notice"><p><strong>Confira o objetivo antes de continuar.</strong> {unsupported} Você pode salvar a entrevista e conferir os documentos.</p><Button onClick={()=>setScreen(a.current_benefit_received===true?'situation':'objective')}>Rever resposta</Button></div></Notice>}

    {screen==='intro'&&<Panel title="Identifique o cálculo" subtitle="Um nome para localizar este trabalho depois."><Field required label="Nome do cálculo" value={draft.name} onChange={name=>update({name})} placeholder="Ex.: Simulação CNIS, revisão de origem"/><p className="form-note">A entrevista pode ser salva como rascunho. Nenhum cálculo existente é substituído automaticamente.</p><div className="form-actions" data-ui-actions><Button onClick={onManual}>Preencher sem assistente</Button><Button primary disabled={!draft.name.trim()} onClick={()=>next('situation')}>Prosseguir<Icon name="arrow" size={16}/></Button></div></Panel>}

    {screen==='situation'&&<>
      <div className="robot-choices" data-ui-stack>
        <Choice title="Não, o segurado não está recebendo benefício" text="Hipótese de novo benefício ou simulação, conforme informação confirmada pelo operador." selected={a.current_benefit_received===false} onClick={()=>answer({current_benefit_received:false},['benefit'])}/>
        <Choice title="Sim, o segurado está recebendo benefício" text="Investigar revisão, restabelecimento ou outro cenário. O motor disponível pode bloquear modalidades não suportadas." selected={a.current_benefit_received===true} onClick={()=>answer({current_benefit_received:true},['benefit'])}/>
        <Choice title="Ainda não tenho a confirmação sobre benefício atual" text="Conferir o CNIS agora e manter essa resposta pendente. A execução continuará bloqueada enquanto faltar confirmação." selected={a.current_benefit_received===null&&a.completed_steps.includes('benefit')} onClick={()=>answer({current_benefit_received:null},['benefit'])}/>
      </div>
      <Notice>Um pedido indeferido no extrato não confirma o recebimento atual. Esta resposta precisa ser declarada por quem conhece o caso.</Notice>
      {a.completed_steps.includes('benefit')&&<Panel title="Deseja importar os dados do INSS?" subtitle="A importação é uma leitura local; ela não confirma os dados nem grava o PDF.">
        {a.import_requested!==true&&<div className="robot-choices" data-ui-stack>
          <Choice title={draft.cnis?'Usar os dados já importados':'Não desejo importar dados do INSS'} text={draft.cnis?'Manter a origem aplicada e evitar uma importação duplicada.':'Continuar com dados e registros declarados manualmente.'} onClick={()=>answer({import_requested:false},['import'],'objective')}/>
          <Choice title="Sim, desejo importar dados do INSS" text="Selecionar o CNIS com remunerações e conferir vínculos, competências e identidade." onClick={()=>{setDraft(withInterviewAnswer(draft,{import_requested:true}));setError('')}}/>
        </div>}
        {a.import_requested===true&&<>
          <div className="robot-import-toolbar"><RobotFace/><div><strong>Vamos conferir a origem.</strong><p>Selecione o documento ou informe seu caminho local.</p></div><Button primary disabled={busy} onClick={()=>void importCnis()}>{busy?<><span className="spinner"/>Lendo CNIS...</>:<>Selecionar CNIS PDF<Icon name="upload" size={16}/></>}</Button></div>
          <details className="details"><summary>Informar caminho do arquivo</summary><div className="robot-path-input"><Field label="Caminho local do CNIS PDF" value={path} onChange={setPath} placeholder="C:\pasta\CNIS.pdf"/><Button disabled={busy||!path.trim()} onClick={()=>void importCnis(true)}>Ler arquivo local</Button></div></details>
          <Button disabled={busy} onClick={()=>{setDraft(withInterviewAnswer(draft,{import_requested:null}));setPending(null)}}>Mudar a resposta de importação</Button>
          {pending&&<><CnisReview cnis={pending}/><div className="import-summary"><div><strong>{pending.salary_entries.length}</strong><span>Lançamentos lidos</span></div><div><strong>{Object.keys(pending.salary_months).length}</strong><span>Competências distintas</span></div><div><strong>{pending.links.length}</strong><span>Vínculos</span></div></div><Notice>Sexo e deficiência continuam não informados. Regime próprio, indicadores e datas finais ausentes não foram validados automaticamente.</Notice><div className="form-actions" data-ui-actions><Button onClick={()=>setPending(null)}>Descartar leitura</Button><Button primary onClick={apply}>Aplicar CNIS e continuar<Icon name="check" size={16}/></Button></div></>}
        </>}
      </Panel>}
    </>}

    {screen==='objective'&&<>
      <div className="robot-choices" data-ui-stack>{PRIMARY_PROCESS.map(id=><Choice key={id} title={PROCESS_TITLES[id]} text={choiceLabel(id)} selected={a.process_kind===id} onClick={()=>chooseProcess(id)}/>)}</div>
      <details className="details"><summary>Outras finalidades ({OTHER_PROCESS.length})</summary><div className="robot-choices" data-ui-stack>{OTHER_PROCESS.map(id=><Choice key={id} title={PROCESS_TITLES[id]} text={choiceLabel(id)} selected={a.process_kind===id} onClick={()=>chooseProcess(id)}/>)}</div></details>
      {a.process_kind&&<Panel title="O que você quer obter?" subtitle="Apenas o novo benefício tem cálculo automático nesta versão."><div className="robot-choices" data-ui-stack>{['Calcular um novo benefício','Calcular somente o tempo de contribuição','Restabelecer um benefício'].map(goal=><Choice key={goal} title={goal} text={goal==='Calcular um novo benefício'?'Conferir dados, períodos, salários, política e memória.':'Sem cálculo automático nesta versão. Você pode registrar o objetivo e conferir os documentos.'} selected={a.goal===goal} onClick={()=>answer({goal},['goal'],'parameters')}/>)}</div></Panel>}
    </>}

    {screen==='parameters'&&<Panel title="Dados iniciais para o benefício" subtitle="Somente os campos que alteram o cálculo."><div className="form-grid" data-ui-stack><Field label="Nome do segurado" required value={draft.identity.full_name??''} onChange={s=>update({identity:{...draft.identity,full_name:s}})}/><Field label="Nascimento do segurado" required value={draft.identity.birth_date?brDate(draft.identity.birth_date):''} onChange={s=>update({identity:{...draft.identity,birth_date:isoDate(s)??s}})} placeholder="DD/MM/AAAA"/><Field label="DIB / data da simulação" required hint="Data de início do benefício. Numa simulação, use hoje ou a data desejada." value={draft.dib} onChange={dib=>update({dib})} placeholder="DD/MM/AAAA"/><Select label="Sexo cadastral do segurado" hint="Do documento. O programa não adivinha pelo nome." value={draft.sex??''} onChange={sex=>update({sex:sex||null})} items={['','Feminino','Masculino','A confirmar']}/><Select label="Deficiência na DIB" hint="Grau reconhecido na data do benefício." value={draft.disability??''} onChange={disability=>update({disability:disability||null})} items={['','Sem deficiência declarada','Leve','Moderada','Grave','A apurar']}/><div className="field"><span>Objetivo informado</span><Badge>{a.goal??'Não informado'}</Badge></div></div><p className="form-note">O CNIS não fornece sexo ou confirmação de deficiência. Não preencher pela aparência do nome. Cenários de teste devem ser registrados como hipóteses, não dados pessoais confirmados.</p>{issues.length>0&&<Notice>Falta confirmar: {issues.join('; ')}.</Notice>}{['Leve','Moderada','Grave'].includes(draft.disability??'')&&<Notice><strong>Cenário PcD.</strong> A aposentadoria PcD pela LC 142/2013 ainda não tem motor neste adaptador: a execução será bloqueada com motivo explícito. A conferência, as notas e os relatórios continuam disponíveis.</Notice>}<div className="form-actions" data-ui-actions><Button onClick={()=>setScreen('objective')}>Voltar ao objetivo</Button><Button primary disabled={issues.length>0} onClick={()=>next('conference',['parameters','periods'])}>Prosseguir para a conferência<Icon name="arrow" size={16}/></Button></div></Panel>}

  {screen==='conference'&&<>
  <ConferenceInterview draft={draft} onFix={setScreen}/>
  <PrepareFromCnis draft={draft} onResult={(prepared)=>{setDraft(prepared);setScreen('result')}} onNotify={onNotify}/>
  <AssistantConsiderationPanel consideration={consideration} onFix={fix}/>
      <div className="conference-tabs" role="group" aria-label="Módulos de conferência">{TABS.map(item=><button key={item.id} type="button" aria-label={item.label} aria-pressed={tab===item.id} aria-controls="conference-panel" onClick={()=>{setTab(item.id);setError('')}}><span>{item.label}</span><small>{item.detail}</small></button>)}</div>
      <div id="conference-panel" role="region" aria-label={TABS.find(item=>item.id===tab)?.label}>
        <CaseWorkbench key={tab} draft={draft} setDraft={setDraft} workspace={workspace} onSave={onSave} onNotify={onNotify} initialStep={TABS.find(item=>item.id===tab)?.step??3} embedded/>
      </div>
      <Notice>Conferir um módulo registra a visita; não valida automaticamente cada vínculo, remuneração ou parâmetro. Cada ajuste invalida o resultado anterior.</Notice>
      <div className="form-actions assistant-conference-note" data-ui-actions><Button onClick={()=>setScreen('parameters')}>Voltar aos dados do segurado</Button><div className="assistant-conference-progress"><span className="form-note">{(['periods','salaries','policy']as ConferenceTab[]).filter(item=>a.completed_steps.includes(item)).length} de 3 módulos abertos. Abra os três para registrar a conferência.</span><Button disabled={!conferenceVisited} title={!conferenceVisited?'Abra os três módulos de conferência para liberar a revisão.':undefined} onClick={()=>setScreen('review')}>Revisar entrevista<Icon name="arrow" size={16}/></Button></div></div>
    </>}

    {screen==='review'&&<div className="split-content">
      <Panel title="O que você informou"><div className="robot-answer-summary">{[['Cálculo',draft.name],['Benefício recebido',a.current_benefit_received===null?'Não confirmado':a.current_benefit_received?'Sim, declarado pelo operador':'Não, declarado pelo operador'],['Finalidade',a.process_kind??'Não informada'],['Objetivo',a.goal??'Não informado'],['Segurado',draft.identity.full_name??'Não informado'],['Sexo / deficiência',(draft.sex??'Pendente')+' · '+(draft.disability??'Pendente')],['DIB',draft.dib],['Fonte',draft.cnis?draft.cnis.links.length+' vínculos / '+Object.keys(draft.cnis.salary_months).length+' competências':'Sem CNIS aplicado']].map(([k,v])=><div key={k}><span>{k}</span><strong>{v}</strong></div>)}</div><label className="checkline" style={{marginTop:22}}><input type="checkbox" checked={a.review_confirmed} onChange={e=>setDraft(withInterviewAnswer(draft,{review_confirmed:e.target.checked}))}/>Conferi as respostas e reconheço as pendências documentais da hipótese.</label><div className="form-actions" data-ui-actions><Button onClick={()=>setScreen('conference')}>Voltar à conferência</Button><Button onClick={()=>void onSave(draft)}>Salvar entrevista</Button><Button primary disabled={!a.review_confirmed} onClick={()=>next('result',['review'])}>Ir para a execução<Icon name="arrow" size={16}/></Button></div></Panel>
      <div><AssistantConsiderationPanel consideration={consideration} onFix={fix}/><Panel title="O que ainda pode mudar a conclusão"><ul className="review-list">{consideration.issues.filter(item=>item.decisive).slice(0,5).map(item=><li key={item.code}>{item.label}</li>)}{consideration.issues.filter(item=>item.decisive).length===0&&<li>Nenhuma pendência decisiva na conferência atual. A execução ainda depende do motor e da revisão profissional.</li>}<li>Revisar a entrevista não confirma automaticamente cada período ou salário.</li>{a.current_benefit_received&&<li>Revisão/restabelecimento a partir de benefício recebido não são suportados pelo adaptador atual.</li>}</ul></Panel></div>
    </div>}

    {screen==='result'&&<>
      <Panel title="Executar com as respostas revisadas" subtitle={draft.result?'A hipótese já foi executada. O resultado abaixo é da última execução.':'O robô envia uma entrada tipada ao motor Rust; nenhum resultado é inventado.'}><div className="head-actions" data-ui-actions><Button primary disabled={busy||!a.review_confirmed||!!unsupported} onClick={()=>void calculate()}>{busy?<><span className="spinner"/>Executando...</>:<>{(draft.result?'Executar novamente':'Executar cálculo da hipótese')}<Icon name="calc" size={16}/></>}</Button><Button disabled={busy} onClick={()=>void onSave(draft)}>Salvar cálculo e entrevista</Button><Button onClick={()=>setScreen('review')}>Voltar à revisão</Button></div>{!a.review_confirmed&&<Notice>Complete e confirme a revisão da entrevista antes de executar.</Notice>}{a.review_confirmed&&!consideration.ready&&!hasFinancialResult(draft)&&<Notice>A execução será bloqueada pelas pendências decisivas. Nenhum valor será inventado para fazer o fluxo passar.</Notice>}{pcdBlocked&&<Notice><div className="assistant-scope-notice"><p><strong>Cenário PcD.</strong> Este caso não roda no motor RGPS: a LC 142/2013 tem um módulo próprio, com premissas declaradas. O cálculo acima continua bloqueado e nenhum valor foi produzido.</p>{onManualAt&&<Button onClick={()=>onManualAt(5)}>Abrir o cenário PcD no editor</Button>}</div></Notice>}</Panel>
      {draft.result&&<ResultView result={draft.result}/>}
      {draft.result&&!hasFinancialResult(draft)&&<AssistantConsiderationPanel consideration={consideration} onFix={fix}/>}
      <section className="assistant-report-center"><h2>Relatórios detalhados em PDF</h2><ReportCenter workspace={workspace} draftCase={draft} selected={draft.id} onSelect={()=>{}} onNotify={onNotify} onChangeCase={c=>setDraft({...c,result:draft.result,assistant:draft.assistant})}/></section>
    </>}

    <footer className="robot-footer"><Button disabled={busy||current===0} onClick={()=>{setScreen(SCREENS[Math.max(0,current-1)]);setError('')}}>Voltar uma etapa</Button><div><Button disabled={busy||!draft.name.trim()} onClick={()=>void onSave(draft)}>Salvar rascunho</Button><Button disabled={busy} onClick={onManual}>Abrir editor de dados</Button></div></footer>
  </div>;
}
