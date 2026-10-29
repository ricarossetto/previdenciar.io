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
    const routes = ['Início', 'Clientes', 'Cálculos', 'Meus cálculos', 'Assistente guiado', 'Editor avançado', 'Tabelas e índices', 'Ferramentas', 'Documentos', 'Relatórios', 'Agenda', 'Configurações', 'Cobertura'];
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
  await page.getByRole('button', { name: 'Reduzir animações', exact: true }).click();
  await page.screenshot({ path: output + '/desktop-dark.png', fullPage: true });
  await page.getByRole('button', { name: 'Alternar tema', exact: true }).click();
  await page.screenshot({ path: output + '/desktop-light.png', fullPage: true });
  await page.getByRole('button', { name: 'Alternar tema', exact: true }).click();
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: output + '/mobile-dark.png', fullPage: true });
  await page.getByRole('button', { name: 'Novo cálculo', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.screenshot({ path: output + '/mobile-dialog.png', fullPage: true });
});
test('Amicro pointer actions have feedback without replaying on text entry', async ({ page }) => {
  await start(page);
  const primary = page.getByRole('button', { name: 'Novo cálculo', exact: true });
  await primary.hover();
  await expect.poll(() => primary.evaluate(el => getComputedStyle(el, '::before').animationName)).toBe('amicro-sheen');
  const card = page.locator('[data-amicro="tilt-card"]').first();
  await card.hover({ position: { x: 35, y: 35 } });
  await expect.poll(() => card.evaluate(el => getComputedStyle(el).transform !== 'none')).toBe(true);
  await page.getByRole('button', { name: 'Cadastrar cliente', exact: true }).click();
  await page.getByRole('dialog').getByLabel('Nome', { exact: true }).fill('Cadastro fictício de teste');
  await expect(page.getByRole('dialog').getByLabel('Nome', { exact: true })).toHaveValue('Cadastro fictício de teste');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
});
test('Amicro honors persisted and system reduced motion', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await start(page);
  await expect(page.getByRole('button', { name: 'Reduzir animações', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('html')).toHaveAttribute('data-amicro-motion', 'off');
  const card = page.locator('[data-amicro="tilt-card"]').first();
  await card.hover({ position: { x: 35, y: 35 } });
  await expect.poll(() => card.evaluate(el => getComputedStyle(el).transform)).toBe('none');
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await expect(page.getByRole('button', { name: 'Reduzir animações', exact: true })).toHaveAttribute('aria-pressed', 'false');
  await page.getByRole('button', { name: 'Reduzir animações', exact: true }).click();
  await page.reload();
  await expect(page.getByRole('button', { name: 'Reduzir animações', exact: true })).toHaveAttribute('aria-pressed', 'true');
});
