import { test, expect } from '@playwright/test';
import { blankWorkspace, blankCase, blankClient } from '../../src/model';
import type { Page } from '@playwright/test';
const output = 'verification/amicro';
async function start(page: Page) {
  const workspace = blankWorkspace();
  workspace.clients = [{ ...blankClient(), id: 1, name: 'Cliente fictício de teste' }];
  workspace.calculations = [{ ...blankCase(), id: 2, name: 'Hipótese sintética de teste' }];
  workspace.next_id = 3;
  await page.addInitScript(w => {
    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: { invoke: async (cmd: string) => {
      if (cmd === 'bootstrap') return { workspace: w, storage_ready: true, message: null };
      if (cmd === 'rea_dashboard') return {};
      if (cmd === 'rea_report') return { output_directory: 'Somente fixture sintética' };
      if (cmd === 'storage_info') return { protected: true, format: 'Fixture UI', path: 'Não é a base real' };
      return null;
    } } });
  }, workspace);
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Início', exact: true })).toBeVisible();
}
for (const width of [1440, 1152, 768, 390, 320]) {
  test('Amicro routes remain readable at ' + width + 'px', async ({ page }) => {
    await page.setViewportSize({ width, height: 1000 });
    await start(page);
    const routes = ['Início', 'Clientes', 'Cálculos', 'Meus cálculos', 'Assistente de cálculo', 'Cálculo previdenciário', 'Tabelas e índices', 'Ferramentas', 'Documentos', 'Relatórios', 'Agenda', 'Configurações', 'Cobertura e validação'];
    for (const route of routes) {
      if (width < 1024) {
        await page.getByRole('button', { name: 'Todas as páginas', exact: true }).click();
        await page.getByRole('dialog').getByRole('button', { name: route, exact: true }).click();
      } else await page.getByRole('navigation', { name: 'Navegação principal' }).getByRole('button', { name: route, exact: true }).click();
      await expect(page.getByRole('heading', { name: route, exact: true })).toBeVisible();
      await expect.poll(() => page.locator('body').evaluate(el => el.scrollWidth <= innerWidth)).toBe(true);
    }
  });
}
test('Amicro desktop and mobile visual evidence with synthetic data', async ({ page }) => {
  await start(page);
  await page.screenshot({ path: output + '/desktop-dark.png', fullPage: true });
  await page.getByRole('button', { name: 'Alternar tema', exact: true }).click();
  await page.screenshot({ path: output + '/desktop-light.png', fullPage: true });
  await page.getByRole('button', { name: 'Alternar tema', exact: true }).click();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: output + '/mobile-dark.png', fullPage: true });
  await page.getByRole('button', { name: /Novo cálculo/ }).first().click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.screenshot({ path: output + '/mobile-dialog.png', fullPage: true });
});
test('Amicro pointer actions have feedback without replaying on text entry', async ({ page }) => {
  await start(page);
  const primary = page.getByRole('button', { name: 'Novo cálculo no escritório', exact: true });
  await primary.hover();
  await expect.poll(() => primary.evaluate(el => getComputedStyle(el, '::before').animationName)).toBe('amicro-sheen');
  await page.getByRole('button', { name: /Cadastrar cliente/ }).click();
  await page.getByRole('dialog').getByLabel('Nome', { exact: true }).fill('Cadastro fictício de teste');
  await expect(page.getByRole('dialog').getByLabel('Nome', { exact: true })).toHaveValue('Cadastro fictício de teste');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
});
test('Amicro honors persisted and system reduced motion', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await start(page);
  await page.getByRole('button', { name: 'Configurações', exact: true }).click();
  const motion = page.getByRole('checkbox', { name: 'Reduzir animações', exact: true });
  await expect(motion).toBeChecked();
  await expect(page.locator('html')).toHaveAttribute('data-amicro-motion', 'off');
  await page.getByRole('button', { name: 'Início', exact: true }).click();
  const primary = page.getByRole('button', { name: 'Novo cálculo no escritório', exact: true });
  await primary.hover();
  await expect.poll(() => primary.evaluate(el => getComputedStyle(el, '::before').animationName)).toBe('none');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.getByRole('button', { name: 'Configurações', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'Reduzir animações', exact: true })).not.toBeChecked();
  await page.getByRole('checkbox', { name: 'Reduzir animações', exact: true }).click();
  await page.reload();
  await page.getByRole('button', { name: 'Configurações', exact: true }).click();
  await expect(page.getByRole('checkbox', { name: 'Reduzir animações', exact: true })).toBeChecked();
});
test('Demo case button opens Previus summary bar and 5-tab TRF3 hub cleanly', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await start(page);
  await page.getByRole('button', { name: 'Carregar caso demonstrativo', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Cálculo previdenciário', level: 1 })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Resumo rápido do cálculo' })).toBeVisible();
  for (const label of ['RMI (17 Espécies)', 'Tempo de Contribuição', 'Atrasados / Liquidação', 'Leitor Documental', 'Índices e Tabelas']) {
    await expect(page.getByRole('tab', { name: label, exact: true })).toBeVisible();
  }
  await expect.poll(() => page.locator('body').evaluate(el => el.scrollWidth <= innerWidth)).toBe(true);
  await page.locator('#fabrica-hub').scrollIntoViewIfNeeded();
  await page.screenshot({ path: 'verification/ui-fabrica-hub.png', fullPage: true });
  await page.getByRole('button', { name: 'Alternar tema', exact: true }).click();
  await page.locator('.previus-summary-bar').scrollIntoViewIfNeeded();
  await page.screenshot({ path: 'verification/ui-fabrica-hub-light.png', fullPage: true });
});
