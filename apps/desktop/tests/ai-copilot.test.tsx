import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AiCopilot, resetAiCopilotCache } from '../src/components/AiCopilot';
import { blankCase, blankWorkspace } from '../src/model';

const mocks = vi.hoisted(() => {
  class Channel<T> {
    onmessage: (value: T) => void = () => {};
    send(value: T) { this.onmessage(value); }
  }
  return { invoke: vi.fn(), Channel };
});
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke, Channel: mocks.Channel }));

const catalog = [
  { id: 'opencode-go', name: 'OpenCode Go - DeepSeek V4.1 Flash', kind: 'openai', baseUrl: 'https://opencode.ai/inference/go/openai/v1', defaultModel: 'deepseek-v4.1-flash', models: ['deepseek-v4.1-flash'], keyRequired: true, note: 'Preset sintético.' },
  { id: 'custom', name: 'Personalizado / gateway', kind: 'openai', baseUrl: '', defaultModel: '', models: [], keyRequired: false, note: 'Endpoint sintético.' },
];

beforeEach(() => {
  localStorage.clear();
  resetAiCopilotCache();
  mocks.invoke.mockReset();
  mocks.invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
    if (command === 'ai_provider_catalog') return catalog;
    if (command === 'ai_key_status') return [{ providerId: 'opencode-go', hasKey: false }, { providerId: 'custom', hasKey: false }];
    if (command === 'ai_data_status') return { skills: 2069, legislation: 64, natjus: 112, natjusText: 112, jurisprudence: 118, ready: true, message: 'Base local sintética pronta.' };
    if (command === 'ai_chat') {
      const channel = args?.onEvent as { send: (event: unknown) => void };
      channel.send({ type: 'tool', name: 'buscar_legislacao', status: 'done', summary: '1 norma local encontrada.', sources: [{ label: 'L1', title: 'Lei 8.213/1991', url: 'https://example.test/lei', kind: 'legislacao' }] });
      channel.send({ type: 'delta', text: 'Resposta sintética com fonte.' });
      channel.send({ type: 'done', finish: 'end_turn', usage: { inputTokens: 12, outputTokens: 5 } });
      return;
    }
    if (command === 'ai_save_key') return;
    throw new Error('Unexpected native command ' + command);
  });
});

async function open(workspace = blankWorkspace()) {
  render(<AiCopilot notify={() => {}} workspace={workspace}/>);
  await waitFor(() => expect(screen.getByLabelText('Provedor')).toHaveValue('opencode-go'));
  return userEvent.setup();
}

describe('Copiloto consent and streaming boundary', () => {
  it('keeps sending blocked until consent and a custom endpoint are configured', async () => {
    const user = await open();
    await user.selectOptions(screen.getByLabelText('Provedor'), 'custom');
    await user.type(screen.getByLabelText('URL base'), 'https://example.test/v1');
    await user.type(screen.getByLabelText('Modelo'), 'synthetic-model');
    await user.type(screen.getByLabelText('Pergunta para o Copiloto'), 'Explique a carência.');
    const send = screen.getByRole('button', { name: /Enviar/ });
    expect(send).toBeDisabled();
    await user.click(screen.getByRole('checkbox', { name: /Permitir enviar/ }));
    expect(send).toBeEnabled();
  });

  it('keeps the draft question when the page is opened again', async () => {
    const first = render(<AiCopilot notify={() => {}} workspace={blankWorkspace()}/>);
    await waitFor(() => expect(screen.getByLabelText('Provedor')).toHaveValue('opencode-go'));
    const user = userEvent.setup();
    await user.type(screen.getByLabelText('Pergunta para o Copiloto'), 'Rascunho da pergunta.');
    first.unmount();
    render(<AiCopilot notify={() => {}} workspace={blankWorkspace()}/>);
    await waitFor(() => expect(screen.getByLabelText('Pergunta para o Copiloto')).toHaveValue('Rascunho da pergunta.'));
  });

  it('shows streamed text and local sources returned by the channel', async () => {
    const user = await open();
    await user.selectOptions(screen.getByLabelText('Provedor'), 'custom');
    await user.type(screen.getByLabelText('URL base'), 'https://example.test/v1');
    await user.type(screen.getByLabelText('Modelo'), 'synthetic-model');
    await user.click(screen.getByRole('checkbox', { name: /Permitir enviar/ }));
    await user.type(screen.getByLabelText('Pergunta para o Copiloto'), 'Pesquise a lei.');
    await user.click(screen.getByRole('button', { name: /Enviar/ }));
    expect(await screen.findByText('Resposta sintética com fonte.')).toBeInTheDocument();
    expect(screen.getByText('Fontes consultadas')).toBeInTheDocument();
    expect(screen.getByText('Lei 8.213/1991')).toBeInTheDocument();
    const call = mocks.invoke.mock.calls.find(([command]) => command === 'ai_chat');
    expect(call?.[1].request.consent).toBe(true);
    expect(call?.[1].request.providerId).toBe('custom');
    expect(call?.[1].request.toolsEnabled).toBe(true);
  });

  it('attaches only the structured case summary, not the case name or identifiers', async () => {
    const workspace = blankWorkspace();
    workspace.calculations = [{ ...blankCase(), id: 1, name: 'Cliente Secreto', kind: 'RGPS', status: 'Dados pendentes', dib: '01/01/2026' }];
    const user = await open(workspace);
    await user.selectOptions(screen.getByLabelText('Provedor'), 'custom');
    await user.type(screen.getByLabelText('URL base'), 'https://example.test/v1');
    await user.type(screen.getByLabelText('Modelo'), 'synthetic-model');
    await user.selectOptions(screen.getByLabelText('Caso'), '1');
    await user.click(screen.getByRole('checkbox', { name: /Anexar este resumo/ }));
    await user.click(screen.getByRole('checkbox', { name: /Permitir enviar/ }));
    await user.type(screen.getByLabelText('Pergunta para o Copiloto'), 'Organize a revisão.');
    await user.click(screen.getByRole('button', { name: /Enviar/ }));
    const call = mocks.invoke.mock.calls.find(([command]) => command === 'ai_chat');
    const context = call?.[1].request.caseContext as string;
    expect(context).toContain('Módulo: RGPS');
    expect(context).toContain('Pendências estruturais:');
    expect(context).not.toContain('Cliente Secreto');
    expect(context).not.toContain('CPF');
  });
});
