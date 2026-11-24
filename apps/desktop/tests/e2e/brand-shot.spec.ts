import { test } from '@playwright/test';
import { blankWorkspace } from '../../src/model';

test('captura a tela inicial para o README', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.addInitScript(w => {
    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: { invoke: async (cmd: string) => {
      if (cmd === 'bootstrap') return { workspace: w, storage_ready: true, message: null };
      if (cmd === 'rea_dashboard') return {};
      if (cmd === 'rea_report') return { output_directory: 'Fixture sintética' };
      if (cmd === 'storage_info') return { protected: true, format: 'Fixture UI', path: 'Não é a base real' };
      return null;
    } } });
  }, blankWorkspace());
  await page.goto('/');
  await page.getByRole('heading', { name: 'Início', exact: true }).waitFor({ timeout: 30000 });
  await page.waitForTimeout(1200);
  await page.screenshot({ path: '../../docs/assets/app-home.png', fullPage: false });
});
