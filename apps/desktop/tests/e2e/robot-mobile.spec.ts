import { test, expect } from '@playwright/test';
import { blankWorkspace, blankCase, blankInterview } from '../../src/model';

test.use({ viewport: { width: 390, height: 844 } });

test('one-question parameters stay visible and actionable at 390px', async ({ page }) => {
  const workspace = blankWorkspace();
  const calculation = {
    ...blankCase(),
    id: 1,
    name: 'Entrevista Sintética Mobile',
    assistant: {
      ...blankInterview(),
      current_benefit_received: false,
      import_requested: false,
      process_kind: 'Simulação do benefício',
      goal: 'Calcular um novo benefício',
      review_confirmed: false,
      completed_steps: ['intro', 'benefit', 'import', 'process', 'goal'],
    },
    identity: {
      ...blankCase().identity,
      full_name: 'Pessoa Fictícia Mobile',
      birth_date: '1962-03-10',
    },
    sex: null,
    disability: null,
  };
  workspace.calculations = [calculation];
  await page.addInitScript((state) => {
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      value: {
        invoke: async (command: string, args: Record<string, unknown>) => {
          if (command === 'bootstrap') return { workspace: state, storage_ready: true, message: null };
          if (command === 'save_workspace') return { ...(args.workspace as object), revision: 1 };
          return null;
        },
      },
    });
  }, workspace);

  await page.goto('/', { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: 'Abrir Entrevista Sintética Mobile', exact: true }).click();

  const sex = page.getByLabel('Sexo cadastral do segurado');
  await expect(sex).toBeVisible();
  const box = await sex.boundingBox();
  expect(box?.y ?? 844).toBeLessThan(740);
  await expect(page.getByRole('button', { name: /Confirmar e avançar/ })).toBeDisabled();
  await page.screenshot({ path: 'verification/ui-robot-parameters-390.png', fullPage: true });
});
