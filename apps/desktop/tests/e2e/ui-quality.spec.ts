import{test,expect}from'@playwright/test';
import{blankWorkspace}from'../../src/model';
async function start(page:import('@playwright/test').Page){
 const workspace=blankWorkspace();
 await page.addInitScript(w=>Object.defineProperty(window,'__TAURI_INTERNALS__',{value:{invoke:async(cmd:string)=>{if(cmd==='bootstrap')return{workspace:w,storage_ready:true,message:null};throw Error('Read-only synthetic UI test: '+cmd);}}}),workspace);
 await page.goto('/',{waitUntil:'domcontentloaded'});await expect(page.getByRole('heading',{name:'Início',level:1,exact:true})).toBeVisible();
}
for(const width of[390,1440]){
 test('visible skip link, real tooltips and keyboard focus at '+width+'px',async({page})=>{
  await page.setViewportSize({width,height:900});await start(page);
  const skip=page.locator('[data-od-id="skip-to-content"]');
  await expect(skip).toBeVisible();await page.keyboard.press('Tab');await expect(skip).toBeFocused();
  await page.keyboard.press('Enter');await expect(page.getByRole('main')).toBeFocused();
  const theme=page.getByRole('button',{name:'Alternar tema',exact:true});
  await theme.hover();const tooltip=page.getByRole('tooltip');await expect(tooltip).toBeVisible();
  const tipId=await tooltip.getAttribute('id');expect((await theme.getAttribute('aria-describedby'))?.split(' ')).toContain(tipId);
  const box=await tooltip.boundingBox();expect(box).not.toBeNull();expect(box!.x).toBeGreaterThanOrEqual(0);expect(box!.y).toBeGreaterThanOrEqual(0);expect(box!.x+box!.width).toBeLessThanOrEqual(width);expect(box!.y+box!.height).toBeLessThanOrEqual(900);
  await page.mouse.move(1,1);await expect(tooltip).toBeHidden();
  for(let n=0;n<40;n++){await page.keyboard.press('Tab');if(await theme.evaluate(el=>el===document.activeElement))break;}
  await expect(theme).toBeFocused();await expect(page.getByRole('tooltip')).toBeVisible();await page.keyboard.press('Escape');await expect(page.getByRole('tooltip')).toBeHidden();await expect(theme).toBeFocused();
  expect(await page.locator('body').evaluate(el=>el.scrollWidth<=innerWidth)).toBe(true);
 });
}
test('calculation choices expose real named buttons and welcome fits before the first interview question',async({page})=>{
 await page.setViewportSize({width:390,height:844});await start(page);
 await page.getByRole('button',{name:'Todas as páginas',exact:true}).click();await page.getByRole('dialog').getByRole('button',{name:'Cálculos do RGPS',exact:true}).click();
 await page.getByRole('button',{name:'Novo cálculo',exact:true}).click();const dialog=page.getByRole('dialog');
 const guided=dialog.getByRole('button',{name:'Usar o assistente de cálculos',exact:true});const manual=dialog.getByRole('button',{name:'Não usar o assistente',exact:true});
 await expect(guided).toHaveText('Usar o assistente de cálculos');await expect(manual).toHaveText('Não usar o assistente');
 await guided.click();const name=page.getByLabel('Nome do cálculo',{exact:true});await expect(name).toBeVisible();const box=await name.boundingBox();expect(box!.y+box!.height).toBeLessThan(844);
 await expect(page.getByRole('group',{name:'Etapas do assistente'})).toHaveCount(0);
 await name.fill('Entrevista fictícia de acessibilidade');await page.getByRole('button',{name:'Prosseguir',exact:true}).click();await expect(page.getByRole('group',{name:'Etapas do assistente'})).toBeVisible();
 await expect(page.getByRole('heading',{name:'O segurado recebe algum benefício previdenciário atualmente?',exact:true})).toBeVisible();
});
