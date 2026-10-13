import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { blankWorkspace, blankCase } from '../../src/model';
import type { Cnis } from '../../src/model';
async function start(page:import('@playwright/test').Page) {
  const workspace=blankWorkspace(),caseData={...blankCase(),id:1,name:'Livro sintético de contribuições'};
  caseData.cnis={metadata:{page_count:1,source_sha256:'c'.repeat(64),extractor:'synthetic'},identity:caseData.identity,
    salary_entries:[1,2].map(index=>({competence:'2020-01',amount_centavos:index===1?10010:20020,indicators:[],link_sequence:null,source:{page:1,line:index,index},excluded_from_validated_domain:false})),
    salary_months:{'2020-01':{count:2,amount_sum_centavos:30030,contains_unresolved_prpps:false}},links:[],benefits:[],warnings:[],findings:[]} as Cnis;
  workspace.calculations=[caseData];workspace.next_id=2;
  await page.addInitScript(seed=>{
    let state=JSON.parse(sessionStorage.getItem('synthetic-contribution-state')??JSON.stringify(seed));
    Object.defineProperty(window,'__TAURI_INTERNALS__',{value:{invoke:async(cmd:string,args:Record<string,unknown>)=>{
      if(cmd==='bootstrap')return {workspace:state,storage_ready:true,message:null};
      if(cmd==='save_workspace'){state={...(args.workspace as typeof seed),revision:state.revision+1};sessionStorage.setItem('synthetic-contribution-state',JSON.stringify(state));return state;}
      if(cmd==='calculate_case')return {status:'blocked',data:null,blockers:[{code:'synthetic_pending',message:'Fixture UI: parâmetros documentais ainda pendentes'}],warnings:[]};
      return null;
    }}});
  },workspace);
  await page.goto('/');await page.getByRole('button',{name:'Cálculos',exact:true}).click();await page.getByRole('row',{name:/Livro sintético/}).getByRole('button',{name:'Abrir',exact:true}).click();await page.getByRole('button',{name:'5. Salários',exact:true}).click();
}
test('editable source contribution is saved and reloads with provenance and documented projection',async({page})=>{
  await start(page);await page.getByRole('button',{name:'Abrir livro de contribuições'}).click();
  await page.getByRole('button',{name:'Editar contribuição source:1'}).click();const dialog=page.getByRole('dialog');
  await dialog.getByLabel('Valor nominal ajustado',{exact:true}).fill('110,11');await dialog.getByLabel('Justificativa documental do ajuste').fill('Documento fictício: retificação de remuneração');await dialog.getByRole('button',{name:'Salvar ajuste'}).click();
  await page.getByLabel('Justificativa documental da ação em lote').fill('Documento fictício de conferência e concomitância');
  for(const action of ['Declarar BRL nos selecionados','Consolidar competência selecionada','Conferir selecionados']){await page.getByLabel('Selecionar lançamentos desta página').check();await page.getByRole('button',{name:action,exact:true}).click();}
  await page.getByRole('button',{name:'Aplicar revisão ao cálculo'}).click();await expect(page.getByText('Aplicado aos salários',{exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Salvar rascunho',exact:true}).click();
  const saved=await page.evaluate(()=>JSON.parse(sessionStorage.getItem('synthetic-contribution-state')!));
  expect(saved.calculations[0].manual_salaries[0].amount).toBe('310.31');expect(saved.calculations[0].cnis.salary_entries.map((e:{amount_centavos:number})=>e.amount_centavos)).toEqual([10010,20020]);expect(saved.calculations[0].contribution_review.history).toHaveLength(5);
  await page.reload();await page.getByRole('button',{name:'Cálculos',exact:true}).click();await page.getByRole('row',{name:/Livro sintético/}).getByRole('button',{name:'Abrir',exact:true}).click();await page.getByRole('button',{name:'5. Salários',exact:true}).click();await expect(page.getByText('Aplicado aos salários',{exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Desfazer última alteração'}).click();await expect(page.getByText('Revisão não aplicada',{exact:true})).toBeVisible();
});
for(const width of [1440,390])test('contribution editor is keyboard accessible and has no measured WCAG violations at '+width,async({page})=>{
  await page.setViewportSize({width,height:1000});await page.emulateMedia({reducedMotion:'reduce'});
  if(width<1024){await page.setViewportSize({width:1440,height:1000});await start(page);await page.setViewportSize({width,height:1000});}else await start(page);
  await page.getByRole('button',{name:'Abrir livro de contribuições'}).click();
  expect(await page.locator('body').evaluate(el=>el.scrollWidth<=innerWidth)).toBe(true);
  const scan=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();expect(scan.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>n.target)}))).toEqual([]);
  await page.getByRole('button',{name:'Editar contribuição source:1'}).click();await expect(page.getByRole('dialog')).toBeVisible();
  const modalScan=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();expect(modalScan.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>n.target)}))).toEqual([]);
  await page.keyboard.press('Escape');await expect(page.getByRole('dialog')).toHaveCount(0);
});
test('new calculation choices have measurable contrast without ignored rules',async({page})=>{
  await start(page);await page.getByRole('button',{name:'Cálculos',exact:true}).click();await page.getByRole('button',{name:'Novo cálculo',exact:true}).click();
  for(const light of [false,true]){
    if(light){await page.keyboard.press('Escape');await page.getByRole('button',{name:'Alternar tema',exact:true}).click();await page.getByRole('button',{name:'Novo cálculo',exact:true}).click();}
    const scan=await new AxeBuilder({page}).withTags(['wcag2a','wcag2aa','wcag21aa','wcag22aa']).analyze();expect(scan.violations.map(v=>({id:v.id,nodes:v.nodes.map(n=>n.target)}))).toEqual([]);
    const text=page.getByText('Origem dos documentos e limites preservados.',{exact:true});expect(await text.evaluate(el=>getComputedStyle(el).backgroundColor)).not.toBe('rgba(0, 0, 0, 0)');
  }
});
