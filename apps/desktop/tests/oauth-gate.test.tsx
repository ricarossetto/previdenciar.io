import { StrictMode } from 'react';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { WebAuthGate } from '../src/auth-web';

// Integração do portão de entrada com um callback sintético. Nenhuma conta,
// nenhum token real e nenhuma chamada de rede: fetch é substituído.
const CHAVE = 'previdenciar.web.oauthAttempt';
const VERIFIER = 'verifier-sintetico-integracao';
const STATE = 'state-sintetico-integracao';

function irParaCallback(search: string) {
  window.history.replaceState({}, '', `/auth/callback${search}`);
}

function guardarTentativa() {
  sessionStorage.setItem(CHAVE, JSON.stringify({ verifier: VERIFIER, state: STATE }));
}

function corpoFalso(status: number, corpo: string) {
  const chamadas: { url: string; body: string | null }[] = [];
  const falso = vi.fn(async (url: string | URL, init?: RequestInit) => {
    chamadas.push({ url: String(url), body: typeof init?.body === 'string' ? init.body : null });
    return {
      ok: status >= 200 && status < 300,
      status,
      text: async () => corpo,
      headers: new Headers({ 'content-type': 'application/json' }),
    } as unknown as Response;
  });
  vi.stubGlobal('fetch', falso);
  return chamadas;
}

beforeEach(() => {
  localStorage.clear();
  sessionStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.history.replaceState({}, '', '/');
});

describe('WebAuthGate no retorno do provedor', () => {
  it('troca o código quando o state confere', async () => {
    guardarTentativa();
    irParaCallback(`?code=abc123&state=${STATE}`);
    const chamadas = corpoFalso(200, '{"id_token":"sintetico.id.token","refresh_token":"r"}');

    render(<WebAuthGate />);

    await act(async () => { await Promise.resolve(); });

    expect(chamadas[0]?.url).toBe('/api/auth/token');
    expect(chamadas[0]?.body).toContain(VERIFIER);
    expect(sessionStorage.getItem(CHAVE)).toBeNull();
    expect(window.location.pathname).toBe('/');
  });

  it('falha na troca limpa a URL e não deixa o código nela', async () => {
    guardarTentativa();
    irParaCallback(`?code=abc123&state=${STATE}`);
    corpoFalso(502, '<!DOCTYPE html><html><body>502</body></html>');

    render(<WebAuthGate />);

    await act(async () => { await Promise.resolve(); });

    const alerta = await screen.findByRole('alert');
    expect(alerta.textContent).toMatch(/indisponível|instantes/i);
    expect(alerta.textContent).not.toMatch(/SyntaxError|\b\d{3}\b/);
    expect(window.location.pathname).toBe('/');
    expect(window.location.search).toBe('');
  });

  it('state divergente não chega a chamar a troca', async () => {
    guardarTentativa();
    irParaCallback('?code=abc123&state=outro');
    const chamadas = corpoFalso(200, '{"id_token":"sintetico.id.token"}');

    render(<WebAuthGate />);

    await act(async () => { await Promise.resolve(); });

    const alerta = await screen.findByRole('alert');
    expect(alerta.textContent).toMatch(/tentativa de login não confere/i);
    expect(chamadas).toHaveLength(0);
    expect(window.location.pathname).toBe('/');
    expect(sessionStorage.getItem(CHAVE)).toBeNull();
  });

  it('reuso do mesmo retorno não repete a troca', async () => {
    guardarTentativa();
    irParaCallback(`?code=abc123&state=${STATE}`);
    corpoFalso(400, '{"error":"Código inválido."}');

    const primeira = render(<WebAuthGate />);
    await act(async () => { await Promise.resolve(); });
    await screen.findByRole('alert');
    primeira.unmount();

    // Segundo carregamento do mesmo endereço: a tentativa já foi consumida.
    const chamadas = corpoFalso(200, '{"id_token":"sintetico.id.token"}');
    irParaCallback(`?code=abc123&state=${STATE}`);
    render(<WebAuthGate />);
    await act(async () => { await Promise.resolve(); });

    expect(chamadas).toHaveLength(0);
    expect(window.location.pathname).toBe('/');
  });

  it('sobrevive ao efeito duplicado do StrictMode sem invalidar a própria tentativa', async () => {
    guardarTentativa();
    irParaCallback(`?code=abc123&state=${STATE}`);
    const chamadas = corpoFalso(200, '{"id_token":"sintetico.id.token"}');

    render(<StrictMode><WebAuthGate /></StrictMode>);

    await act(async () => { await Promise.resolve(); });

    // Uma única troca, sem mensagem de tentativa divergente.
    expect(chamadas.filter((c) => c.url === '/api/auth/token')).toHaveLength(1);
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('autentica diretamente com credenciais e assinatura do Atrium', async () => {
    const chamadas = corpoFalso(200, '{"id_token":"eyJhbGciOiJIUzI1NiJ9.eyJleHAiOjQxMDI0NDQ4MDB9.sig"}');

    render(<WebAuthGate />);
    await act(async () => { await Promise.resolve(); });

    const userInput = await screen.findByPlaceholderText(/usuario ou email/i);
    const passInput = await screen.findByPlaceholderText(/Sua senha do Atrium/i);
    fireEvent.change(userInput, { target: { value: 'advogado@atrium.adv.br' } });
    fireEvent.change(passInput, { target: { value: 'SenhaForte123!' } });

    const submitBtn = screen.getByRole('button', { name: /Entrar com conta Atrium/i });
    fireEvent.click(submitBtn);

    await act(async () => { await Promise.resolve(); });

    expect(chamadas.some((c) => c.url === '/api/auth/atrium-login')).toBe(true);
    const loginCall = chamadas.find((c) => c.url === '/api/auth/atrium-login');
    expect(loginCall?.body).toContain('advogado@atrium.adv.br');
  });
});
