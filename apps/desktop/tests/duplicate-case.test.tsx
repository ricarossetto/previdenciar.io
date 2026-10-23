import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from '../src/App';
import { blankCase, blankWorkspace } from '../src/model';

const invoke = vi.hoisted(() => vi.fn());

vi.mock('@tauri-apps/api/core', () => ({ invoke }));

beforeEach(() => {
  invoke.mockReset();
});

describe('duplicating a case', () => {
  it('allocates fresh ids for manual links and salaries', async () => {
    const w = blankWorkspace();
    w.calculations = [{
      ...blankCase(),
      id: 1,
      name: 'Caso com linhas manuais',
      kind: 'RGPS',
      manual_links: [{ id: 2, employer: 'Empregador sintético', start_date: '2020-01-01', end_date: '2020-12-31', regime: 'RGPS', indicators: '', confirmed: true }],
      manual_salaries: [{ id: 3, competence: '2020-01', amount: '1000.00', indicators: '', confirmed: true }],
    }];
    w.next_id = 4;
    invoke.mockImplementation(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'bootstrap') return { workspace: w, storage_ready: true, message: null };
      if (cmd === 'save_workspace') return { ...(args?.workspace as object), revision: 2 };
      throw new Error('Unexpected native command ' + cmd);
    });
    render(<App />);
    await screen.findByRole('heading', { name: 'Início' });
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Cálculos', exact: true }));
    await user.click(screen.getByRole('button', { name: 'Duplicar' }));
    await waitFor(() => expect(invoke.mock.calls.some((call) => call[0] === 'save_workspace')).toBe(true));
    const saved = invoke.mock.calls.find((call) => call[0] === 'save_workspace')?.[1].workspace as {
      calculations: { id: number; manual_links: { id: number }[]; manual_salaries: { id: number }[] }[];
    };
    expect(saved.calculations).toHaveLength(2);
    const ids = saved.calculations.flatMap((c) => [c.id, ...c.manual_links.map((row) => row.id), ...c.manual_salaries.map((row) => row.id)]);
    expect(new Set(ids).size).toBe(ids.length);
    expect(saved.calculations[1].manual_links[0].id).toBeGreaterThan(3);
    expect(saved.calculations[1].manual_salaries[0].id).toBeGreaterThan(saved.calculations[1].manual_links[0].id);
  });
});
