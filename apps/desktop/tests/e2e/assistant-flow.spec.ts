import{test,expect}from'@playwright/test';
import{blankCase,blankInterview,blankWorkspace,emptyIdentity}from'../../src/model';
import type{Calculation,Workspace}from'../../src/model';
function workspace():Workspace{
  const w=blankWorkspace();
  const c:Calculation={...blankCase(),id:1,name:'Caso sintético de conferência',status:'Dados revisados',identity:{...emptyIdentity(),full_name:'Pessoa Fictícia de Teste',birth_date:'1960-01-01'},sex:'Feminino',disability:'Sem deficiência declarada',dib:'2024-01-01',
    assistant:{...blankInterview(),mode:'assisted',current_benefit_received:false,import_requested:true,process_kind:'Simulação do benefício',goal:'Calcular um novo benefício',review_confirmed:false,completed_steps:['intro','benefit','import','process','goal','parameters','periods']},
    cnis:{metadata:{page_count:1,source_sha256:'synthetic',extractor:'synthetic'},identity:{...emptyIdentity(),full_name:'Pessoa Fictícia de Teste',birth_date:'1960-01-01'},
      links:[{sequence:1,raw_employer:'EMPREGADOR FICTÍCIO',affiliation:'empregado',start_date:'2020-01-01',end_date:'2023-12-31',prpps:false,indicators:[],reported_last_salary_month:null,inferred_last_salary_end:null,source:{page:1}}],
      salary_entries:[{competence:'2022-01',amount_centavos:200000,indicators:[],link_sequence:1,source:{page:1,line:1,index:1},excluded_from_validated_domain:false}],
      salary_months:{'2022-01':{count:1,amount_sum_centavos:200000,contains_unresolved_prpps:false}},benefits:[],warnings:[],findings:[]},
    manual_links:[{id:1,employer:'EMPREGADOR FICTÍCIO',start_date:'2020-01-01',end_date:'2023-12-31',regime:'RGPS',indicators:'',confirmed:true,source_review:null}],
    manual_salaries:[{id:2,competence:'2022-01',amount:'2000.00',indicators:'',confirmed:true}],
    options:{...blankCase().options,rule_policy:JSON.stringify({version:'rgps-core-observational-v1',review_evidence:'DOC-SINTETICO',manual_link_kind:'empregado',manual_currency:'BRL',native_options:{corrigirSalarios:false,descarte:false,divisorMinimo:false,aplicarTetoVigenteDer:false},exclude_imported_source:true,source_exclusion_evidence:'DOC-SINTETICO-2'})}};
  w.calculations=[c];return w;
}
async function start(page:import('@playwright/test').Page){
  await page.addInitScript((w:Workspace)=>Object.defineProperty(window,'__TAURI_INTERNALS__',{value:{invoke:async(cmd:string)=>{if(cmd==='bootstrap')return{workspace:w,storage_ready:true,message:null};throw Error('Read-only synthetic UI test: '+cmd);}}}),workspace());
  await page.goto('/',{waitUntil:'domcontentloaded'});await expect(page.getByRole('heading',{name:'Início',level:1,exact:true})).toBeVisible();
}
async function openAssistant(page:import('@playwright/test').Page,width:number,testInfo:import('@playwright/test').TestInfo){
  await page.setViewportSize({width,height:900});await start(page);
  if(width<1024){await page.getByRole('button',{name:'Todas as páginas',exact:true}).click();await page.getByRole('dialog').getByRole('button',{name:'Cálculos do RGPS',exact:true}).click();}
  else await page.getByRole('button',{name:'Cálculos do RGPS',exact:true}).click();
  await page.getByRole('button',{name:'Abrir',exact:true}).click();
  await expect(page.getByText('O que entra no cálculo')).toBeVisible();
  await expect(page.getByText('Pronto para o motor')).toBeVisible();
  expect(await page.locator('body').evaluate(el=>el.scrollWidth<=innerWidth)).toBe(true);
  await page.screenshot({path:testInfo.outputPath(`assistant-conference-${width}.png`),fullPage:true});
}
for(const width of[390,1440]){
  test('assistant opens at the conference hub with source, adjustments and used data at '+width+'px',async({page},testInfo)=>{
    await openAssistant(page,width,testInfo);
    for(const label of['Importado da fonte','Ajustado pelo operador','Preparado para execução','Conferir períodos','Conferir remunerações','Conferir opções da RMI'])await expect(page.getByText(label,{exact:true})).toBeVisible();
    await expect(page.getByRole('button',{name:'Revisar entrevista'})).toBeDisabled();
    await page.getByRole('button',{name:'Conferir remunerações'}).click();
    await page.getByRole('button',{name:'Conferir opções da RMI'}).click();
    await expect(page.getByRole('button',{name:'Revisar entrevista'})).toBeEnabled();
  });
}
test('report center exposes the seven-family batch without generating a fake preview',async({page})=>{
  await page.setViewportSize({width:1440,height:1000});await start(page);
  await page.getByRole('button',{name:'Relatórios',exact:true}).click();
  await page.getByLabel('Selecionar cálculo').selectOption('1');
  await expect(page.getByText('Exportar vários relatórios')).toBeVisible();
  await page.getByRole('button',{name:'Selecionar todos'}).click();
  await expect(page.getByRole('button',{name:'Exportar 8 PDF(s)'})).toBeEnabled();
  expect(await page.locator('body').evaluate(el=>el.scrollWidth<=innerWidth)).toBe(true);
});
test('report center offers an editable title and presentation choices',async({page},testInfo)=>{
  await page.setViewportSize({width:1440,height:1000});await start(page);
  await page.getByRole('button',{name:'Relatórios',exact:true}).click();
  await page.getByLabel('Selecionar cálculo').selectOption('1');
  await expect(page.getByText('Apresentação e título')).toBeVisible();
  await page.getByLabel('Título do relatório').fill('RMI, revisão de origem (2026)');
  await expect(page.getByRole('button',{name:'Aplicar título ao cálculo'})).toBeEnabled();
  await page.getByRole('checkbox',{name:'Incluir o anexo técnico de origem (relatório completo)'}).uncheck();
  await page.getByRole('checkbox',{name:'Incluir a conferência de qualidade por vínculo'}).uncheck();
  expect(await page.locator('body').evaluate(el=>el.scrollWidth<=innerWidth)).toBe(true);
  await page.screenshot({path:testInfo.outputPath('report-center-presentation.png'),fullPage:true});
});
