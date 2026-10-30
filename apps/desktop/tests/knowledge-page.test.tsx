import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { KnowledgePage } from '../src/components/KnowledgePage';

const mocks = vi.hoisted(() => {
  class Channel<T> {
    onmessage: (value: T) => void = () => {};
    send(value: T) { this.onmessage(value); }
  }
  return { invoke: vi.fn(), Channel };
});
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke, Channel: mocks.Channel }));

beforeEach(() => {
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation(async (command: string) => {
    if (command === 'ai_data_status') return { skills: 2069, legislation: 64, natjus: 112, natjusText: 112, jurisprudence: 118, ready: true, message: 'Base local pronta.' };
    if (command === 'ai_knowledge_search') return {
      summary: '1 habilidade local encontrada.',
      sources: [{ label: 'H1', title: 'Calculo de RMI', url: null, kind: 'habilidade' }],
      value: { resultados: [{ ref: 'H1', id: 'calculo-rmi', titulo: 'Calculo de RMI', tipo: 'previdenciario / calculo' }] },
    };
    if (command === 'ai_knowledge_read') return {
      summary: 'Habilidade lida.',
      sources: [{ label: 'H', title: 'Calculo de RMI', url: null, kind: 'habilidade' }],
      value: { titulo: 'Calculo de RMI', conteudo: 'Conteudo local da habilidade sintetica.' },
    };
    throw new Error('Unexpected native command ' + command);
  });
});

describe('Knowledge base page', () => {
  it('searches and reads a local block without calling a model', async () => {
    const user = userEvent.setup();
    render(<KnowledgePage notify={() => {}}/>);
    expect(await screen.findByText('Base importada')).toBeInTheDocument();
    const input = screen.getByLabelText('Pesquisar em Habilidades');
    await user.type(input, 'RMI');
    await user.click(screen.getByRole('button', { name: 'Buscar' }));
    expect(await screen.findByText('Calculo de RMI')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /Calculo de RMI/ }));
    expect(await screen.findByText('Conteudo local da habilidade sintetica.')).toBeInTheDocument();
    expect(mocks.invoke.mock.calls.some(([command]) => command === 'ai_knowledge_search')).toBe(true);
    expect(mocks.invoke.mock.calls.some(([command]) => command === 'ai_knowledge_read')).toBe(true);
    expect(mocks.invoke.mock.calls.some(([command]) => command === 'ai_chat')).toBe(false);
  });

  it('switches block and keeps the search local', async () => {
    const user = userEvent.setup();
    render(<KnowledgePage notify={() => {}}/>);
    await waitFor(() => expect(screen.getByText('Base importada')).toBeInTheDocument());
    await user.click(screen.getByRole('tab', { name: /Legislação/ }));
    expect(screen.getByLabelText('Pesquisar em Legislação').getAttribute('placeholder')).toContain('Lei 8.213');
  });
});
