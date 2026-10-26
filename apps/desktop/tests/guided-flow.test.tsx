import{describe,it,expect,vi,beforeEach}from'vitest';import{render,screen,within}from'@testing-library/react';import userEvent from'@testing-library/user-event';import App from'../src/App';import{blankWorkspace}from'../src/model';import type{Calculation,Cnis}from'../src/model';const invoke=vi.hoisted(()=>vi.fn());vi.mock('@tauri-apps/api/core',()=>({invoke}));

/// Fluxo guiado completo com IPC sintético: entrada, entrevista, CNIS, conferência
/// e preparo em um clique. Nenhum dado real entra neste arquivo.
function fixtureCnis():Cnis{return{metadata:{page_count:1,source_sha256:'sintetico',extractor:'synthetic'},identity:{full_name:'Pessoa Fictícia de Teste',cpf:null,nit:null,birth_date:'1962-03-10',mother_name:null},salary_entries:[{competence:'2020-01',amount_centavos:200000,indicators:[],link_sequence:1,source:{page:1,line:1,index:1},excluded_from_validated_domain:false},{competence:'2020-02',amount_centavos:210000,indicators:[],link_sequence:1,source:{page:1,line:2,index:2},excluded_from_validated_domain:false}],salary_months:{'2020-01':{count:1,amount_sum_centavos:200000,contains_unresolved_prpps:false},'2020-02':{count:1,amount_sum_centavos:210000,contains_unresolved_prpps:false}},links:[{sequence:1,raw_employer:'EMPREGADOR FICTÍCIO',start_date:'2019-01-01',end_date:'2020-02-29',prpps:false,indicators:[],reported_last_salary_month:'2020-02',inferred_last_salary_end:null,source:{page:1}}],benefits:[],findings:[],warnings:[]}}

function preparedFrom(calculation:Calculation):Calculation{
  return {...calculation,status:'Revisão pendente',options:{...calculation.options,rule_policy:JSON.stringify({version:'rgps-core-observational-v1',exclude_imported_source:true,source_exclusion_evidence:'CNIS conferido no teste'})}};
}

beforeEach(()=>{invoke.mockReset();invoke.mockImplementation(async(cmd:string,args?:Record<string,unknown>)=>{if(cmd==='bootstrap')return{workspace:blankWorkspace(),storage_ready:true,message:null};if(cmd==='save_workspace')return{...(args?.workspace as object),revision:1};if(cmd==='pick_import_cnis'||cmd==='import_cnis_path')return fixtureCnis();if(cmd==='prepare_case_from_cnis')return{calculation:preparedFrom(args?.calculation as Calculation),notes:['Vínculos revisados a partir do CNIS.']};if(cmd==='calculate_case')return{status:'experimental',data:{native_result:{resumo:{rmi:'1600.00'},memoria:[]}},warnings:[]};throw new Error('Unexpected command '+cmd)});});

async function toConference(){
  render(<App/>);await screen.findByRole('heading',{name:'Início'});const u=userEvent.setup();
  await u.click(screen.getByRole('button',{name:'Cálculos do RGPS',exact:true}));
  await u.click(screen.getByRole('button',{name:'Novo cálculo',exact:true}));
  await u.click(screen.getByRole('button',{name:'Usar o assistente de cálculos',exact:true}));
  await u.type(screen.getByLabelText('Nome do cálculo'),'Simulação Sintética do Assistente');
  await u.click(screen.getByRole('button',{name:'Prosseguir',exact:true}));
  await u.click(screen.getByRole('button',{name:/Não, o segurado não está recebendo benefício/}));
  await u.click(screen.getByRole('button',{name:/Sim, desejo importar dados do INSS/}));
  await u.click(screen.getByRole('button',{name:/Selecionar CNIS PDF/}));
  await u.click(await screen.findByRole('button',{name:'Aplicar CNIS e continuar'}));
  await u.click(screen.getByRole('button',{name:/Simular um novo benefício/}));
  await u.click(screen.getByRole('button',{name:/Calcular um novo benefício/}));
  await u.selectOptions(screen.getByLabelText('Sexo cadastral do segurado'),'Feminino');
  await u.selectOptions(screen.getByLabelText('Deficiência na DIB'),'Grave');
  expect(screen.getByText(/Cenário PcD/)).toBeInTheDocument();
  await u.click(screen.getByRole('button',{name:/Prosseguir para a conferência/}));
  return u;
}

describe('conferência do assistente a partir do CNIS',()=>{
it('mostra as respostas da entrevista e um único caminho para calcular',async()=>{
  await toConference();
  const answers=screen.getByRole('region',{name:'Respostas da entrevista'});
  expect(within(answers).getByText('Não recebe benefício (declarado)')).toBeInTheDocument();
  expect(within(answers).getByText('Calcular um novo benefício')).toBeInTheDocument();
  expect(screen.queryByText('Situação de benefício atual não confirmada.')).toBeNull();
  expect(screen.getByRole('heading',{name:'Montar a hipótese a partir do CNIS'})).toBeInTheDocument();
  expect(screen.getByText(/Um clique resolve as pendências/)).toBeInTheDocument();
  expect(screen.queryByRole('button',{name:/Corrigir: Fonte CNIS/})).toBeNull();
});

it('prepara e calcula com a resposta da entrevista preservada',async()=>{
  const u=await toConference();
  await u.type(screen.getByLabelText('Referência documental da conferência'),'CNIS conferido no teste');
  await u.click(screen.getByRole('button',{name:'Preparar e calcular'}));
  const prepare=invoke.mock.calls.find(call=>call[0]==='prepare_case_from_cnis');
  expect(prepare).toBeTruthy();
  const sent=prepare?.[1].calculation as Calculation;
  expect(sent.assistant?.current_benefit_received).toBe(false);
  expect(sent.assistant?.goal).toBe('Calcular um novo benefício');
  expect(prepare?.[1].reference).toBe('CNIS conferido no teste');
  const calculate=invoke.mock.calls.find(call=>call[0]==='calculate_case');
  expect((calculate?.[1].calculation as Calculation).assistant?.current_benefit_received).toBe(false);
  expect(await screen.findByRole('heading',{name:'Executar com as respostas revisadas'})).toBeInTheDocument();
  expect(screen.getByText('R$ 1.600,00')).toBeInTheDocument();
});
});
