import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MotionProvider, useMotionPolicy } from '../src/components/amicro/motion';
import { DashboardOverview } from '../src/components/amicro/DashboardOverview';
import { Button } from '../src/components/ui';
import { blankWorkspace, blankCase, blankClient } from '../src/model';

afterEach(() => { cleanup(); localStorage.clear(); vi.unstubAllGlobals(); });
function Control() {
  const { reduced, toggle } = useMotionPolicy();
  return <button aria-pressed={reduced} onClick={toggle}>Reduzir animações</button>;
}
describe('Amicro integration', () => {
  it('uses only active local records for dashboard indicators', () => {
    const workspace = blankWorkspace();
    workspace.clients = [{ ...blankClient(), id: 1 }, { ...blankClient(), id: 2, archived: true }];
    workspace.calculations = [{ ...blankCase(), id: 3 }, { ...blankCase(), id: 4, archived: true }];
    workspace.appointments = [{ id: 5, title: 'Compromisso fictício', date: '2026-10-13', time: '09:00', client_id: null, notes: '', completed: false }];
    render(<MotionProvider><DashboardOverview workspace={workspace} onStart={() => {}} onClient={() => {}} onReports={() => {}} /></MotionProvider>);
    const metrics = screen.getByLabelText('Indicadores da base local').querySelectorAll('strong');
    expect(Array.from(metrics, item => item.textContent)).toEqual(['1', '1', '1', '1']);
    expect(screen.queryByText(/RMI/)).not.toBeInTheDocument();
  });
  it('routes each shortcut once without altering workspace values', async () => {
    const onStart = vi.fn(), onClient = vi.fn(), onReports = vi.fn();
    const workspace = blankWorkspace(), before = JSON.stringify(workspace);
    render(<MotionProvider><DashboardOverview workspace={workspace} {...{ onStart, onClient, onReports }} /></MotionProvider>);
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Abrir entrevista' }));
    await user.click(screen.getByRole('button', { name: 'Cadastrar cliente' }));
    await user.click(screen.getByRole('button', { name: 'Consultar relatórios' }));
    expect(onStart).toHaveBeenCalledTimes(1); expect(onClient).toHaveBeenCalledTimes(1); expect(onReports).toHaveBeenCalledTimes(1);
    expect(JSON.stringify(workspace)).toBe(before);
  });
  it('persists optional reduced motion without modifying the business settings', async () => {
    render(<MotionProvider><Control /></MotionProvider>);
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'false');
    await userEvent.setup().click(screen.getByRole('button'));
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true');
    expect(localStorage.getItem('previus.amicro.reduce-motion')).toBe('true');
    expect(document.documentElement.dataset.amicroMotion).toBe('off');
    cleanup();
    render(<MotionProvider><Control /></MotionProvider>);
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true');
  });
  it('never overrides the operating system reduced-motion preference', async () => {
    vi.stubGlobal('matchMedia', vi.fn(() => ({ matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() })));
    render(<MotionProvider><Control /></MotionProvider>);
    await userEvent.setup().click(screen.getByRole('button'));
    await userEvent.setup().click(screen.getByRole('button'));
    expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true');
    expect(document.documentElement.dataset.amicroMotion).toBe('off');
  });
  it('preserves disabled controls and their accessible names', async () => {
    const onClick = vi.fn();
    render(<MotionProvider><Button disabled primary aria-label="Ação indisponível" onClick={onClick}>Não executar</Button></MotionProvider>);
    await userEvent.setup().click(screen.getByRole('button', { name: 'Ação indisponível' }));
    expect(onClick).not.toHaveBeenCalled();
  });
});
