import { describe, expect, it } from 'vitest';
import {
  consumeAttempt, describeCallbackError, readCallback, resolveCallback, sameState,
} from '../src/oauth-callback';
import type { CallbackRead, LoginAttempt } from '../src/oauth-callback';

// Nenhum código, token ou dado de cliente real: valores sintéticos.

// Armazenamento em memória com a mesma superfície usada pelo consumo.
function armazenamento(inicial?: string) {
  let valor = inicial ?? null;
  return {
    getItem: () => valor,
    removeItem: () => { valor = null; },
    conteudo: () => valor,
  };
}

const TENTATIVA: LoginAttempt = { verifier: 'verifier-sintetico', state: 'state-sintetico' };

function consumidor(tentativa: LoginAttempt | null) {
  let chamadas = 0;
  return {
    consumir: () => { chamadas += 1; return tentativa; },
    chamadas: () => chamadas,
  };
}

describe('readCallback', () => {
  it('reconhece retorno bem-sucedido com code e state', () => {
    const r = readCallback('/auth/callback', new URLSearchParams('code=abc123&state=xyz'));
    expect(r).toMatchObject({ kind: 'code', code: 'abc123', state: 'xyz' });
  });

  it('devolve state nulo quando o provedor não envia', () => {
    const r = readCallback('/auth/callback', new URLSearchParams('code=abc123'));
    expect(r).toMatchObject({ kind: 'code', state: null });
  });

  it('reconhece retorno de erro do provedor', () => {
    const r = readCallback('/auth/callback', new URLSearchParams('error=access_denied'));
    expect(r.kind).toBe('error');
  });

  it('prioriza o erro quando vier erro e code juntos', () => {
    const r = readCallback('/auth/callback', new URLSearchParams('error=access_denied&code=abc123'));
    expect(r.kind).toBe('error');
  });

  it('trata callback sem code e sem erro como inválido', () => {
    expect(readCallback('/auth/callback', new URLSearchParams('')).kind).toBe('empty');
  });

  it('ignora code vazio', () => {
    expect(readCallback('/auth/callback', new URLSearchParams('code=')).kind).toBe('empty');
  });

  it('não interfere em rota que não é o callback', () => {
    expect(readCallback('/', new URLSearchParams('code=abc123')).kind).toBe('none');
  });
});

describe('sameState', () => {
  it('aprova state idêntico', () => {
    expect(sameState('abc', 'abc')).toBe(true);
  });

  it('reprova state divergente', () => {
    expect(sameState('abc', 'abd')).toBe(false);
  });

  it('reprova ausência de qualquer lado', () => {
    expect(sameState(null, 'abc')).toBe(false);
    expect(sameState('abc', null)).toBe(false);
    expect(sameState(null, null)).toBe(false);
  });

  it('reprova comprimento diferente sem lançar', () => {
    expect(sameState('abc', 'abcd')).toBe(false);
  });
});

describe('consumeAttempt', () => {
  it('devolve a tentativa guardada', () => {
    const store = armazenamento(JSON.stringify(TENTATIVA));
    expect(consumeAttempt(store, 'chave')).toEqual(TENTATIVA);
  });

  it('apaga a tentativa ao consumir', () => {
    const store = armazenamento(JSON.stringify(TENTATIVA));
    consumeAttempt(store, 'chave');
    expect(store.conteudo()).toBeNull();
  });

  it('apaga mesmo quando o conteúdo é inválido', () => {
    const store = armazenamento('{nao-json');
    expect(consumeAttempt(store, 'chave')).toBeNull();
    expect(store.conteudo()).toBeNull();
  });

  it('recusa registro sem verifier ou sem state', () => {
    expect(consumeAttempt(armazenamento('{"state":"s"}'), 'chave')).toBeNull();
    expect(consumeAttempt(armazenamento('{"verifier":"v"}'), 'chave')).toBeNull();
    expect(consumeAttempt(armazenamento('{"verifier":"","state":"s"}'), 'chave')).toBeNull();
  });

  it('devolve null quando não há tentativa', () => {
    expect(consumeAttempt(armazenamento(), 'chave')).toBeNull();
  });
});

describe('resolveCallback', () => {
  const codeCom = (state: string | null): CallbackRead => ({ kind: 'code', code: 'abc123', state });

  it('prossegue com state correspondente e devolve o verifier', () => {
    const c = consumidor(TENTATIVA);
    const d = resolveCallback(codeCom('state-sintetico'), c.consumir);
    expect(d).toMatchObject({ kind: 'prosseguir', code: 'abc123', verifier: 'verifier-sintetico' });
    expect(c.chamadas()).toBe(1);
  });

  it('recusa state divergente e consome a tentativa mesmo assim', () => {
    const c = consumidor(TENTATIVA);
    const d = resolveCallback(codeCom('outro-state'), c.consumir);
    expect(d).toMatchObject({ kind: 'mensagem' });
    expect(d.kind === 'mensagem' && d.texto).toMatch(/tentativa de login não confere/i);
    expect(c.chamadas()).toBe(1);
  });

  it('recusa state ausente', () => {
    const c = consumidor(TENTATIVA);
    const d = resolveCallback(codeCom(null), c.consumir);
    expect(d).toMatchObject({ kind: 'mensagem' });
    expect(c.chamadas()).toBe(1);
  });

  it('recusa quando não existe tentativa guardada', () => {
    const c = consumidor(null);
    const d = resolveCallback(codeCom('state-sintetico'), c.consumir);
    expect(d.kind === 'mensagem' && d.texto).toMatch(/tentativa de login não confere/i);
    expect(c.chamadas()).toBe(1);
  });

  it('consome a tentativa no retorno de erro do provedor', () => {
    const c = consumidor(TENTATIVA);
    const d = resolveCallback({ kind: 'error', reason: 'access_denied', state: 'state-sintetico' }, c.consumir);
    expect(d.kind === 'mensagem' && d.texto).toMatch(/cancel/i);
    expect(c.chamadas()).toBe(1);
  });

  it('consome a tentativa no retorno vazio', () => {
    const c = consumidor(TENTATIVA);
    const d = resolveCallback({ kind: 'empty', state: null }, c.consumir);
    expect(d.kind === 'mensagem' && d.texto).toMatch(/não retornou um código válido/i);
    expect(c.chamadas()).toBe(1);
  });

  it('não consome nem limpa URL fora do callback', () => {
    const c = consumidor(TENTATIVA);
    const d = resolveCallback({ kind: 'none' }, c.consumir);
    expect(d).toEqual({ kind: 'nenhum', limparUrl: false });
    expect(c.chamadas()).toBe(0);
  });

  it('marca limpeza de URL em todos os caminhos do callback', () => {
    expect(resolveCallback(codeCom('state-sintetico'), consumidor(TENTATIVA).consumir).limparUrl).toBe(true);
    expect(resolveCallback(codeCom('divergente'), consumidor(TENTATIVA).consumir).limparUrl).toBe(true);
    expect(resolveCallback({ kind: 'empty', state: null }, consumidor(TENTATIVA).consumir).limparUrl).toBe(true);
  });

  it('não reproduz o motivo bruto nem o code na mensagem', () => {
    const bruto = '<script>alert(1)</script>';
    const d = resolveCallback({ kind: 'error', reason: bruto, state: null }, consumidor(null).consumir);
    const texto = d.kind === 'mensagem' ? d.texto : '';
    expect(texto).not.toContain('<script>');
    expect(texto).not.toContain(bruto);
    expect(texto).not.toContain('abc123');
  });

  it('reuso: a segunda tentativa não encontra mais o registro', () => {
    const store = armazenamento(JSON.stringify(TENTATIVA));
    const primeira = resolveCallback(codeCom('state-sintetico'), () => consumeAttempt(store, 'chave'));
    expect(primeira.kind).toBe('prosseguir');
    const segunda = resolveCallback(codeCom('state-sintetico'), () => consumeAttempt(store, 'chave'));
    expect(segunda).toMatchObject({ kind: 'mensagem' });
  });
});

describe('describeCallbackError', () => {
  it('explica o cancelamento pelo usuário sem culpar o sistema', () => {
    const t = describeCallbackError('access_denied');
    expect(t).toMatch(/cancel/i);
    expect(t).not.toMatch(/erro desconhecido/i);
  });

  it('trata sessão de login expirada', () => {
    expect(describeCallbackError('login_required')).toMatch(/expirou|tempo/i);
  });

  it('trata indisponibilidade do provedor com orientação de nova tentativa', () => {
    expect(describeCallbackError('temporarily_unavailable')).toMatch(/tente de novo|instantes/i);
  });

  it('nunca devolve o texto do provedor nem o código cru', () => {
    const malicioso = '<script>alert(1)</script>';
    const t = describeCallbackError(malicioso);
    expect(t).not.toContain('<script>');
    expect(t).not.toContain('access_denied');
    expect(t).not.toContain(malicioso);
    expect(t).toMatch(/não foi possível concluir o login/i);
  });

  it('devolve mensagem padrão para erro ausente', () => {
    expect(describeCallbackError(null)).toMatch(/não foi possível concluir o login/i);
  });
});

describe('describeCallbackError', () => {
  it('explica o cancelamento pelo usuário sem culpar o sistema', () => {
    const t = describeCallbackError('access_denied');
    expect(t).toMatch(/cancel/i);
    expect(t).not.toMatch(/erro desconhecido/i);
  });

  it('trata sessão de login expirada', () => {
    expect(describeCallbackError('login_required')).toMatch(/expirou|tempo/i);
  });

  it('trata indisponibilidade do provedor com orientação de nova tentativa', () => {
    expect(describeCallbackError('temporarily_unavailable')).toMatch(/tente de novo|instantes/i);
  });

  it('nunca devolve o texto do provedor nem o código cru', () => {
    const malicioso = '<script>alert(1)</script>';
    const t = describeCallbackError(malicioso);
    expect(t).not.toContain('<script>');
    expect(t).not.toContain('access_denied');
    expect(t).not.toContain(malicioso);
    expect(t).toMatch(/não foi possível concluir o login/i);
  });

  it('devolve mensagem padrão para erro ausente', () => {
    expect(describeCallbackError(null)).toMatch(/não foi possível concluir o login/i);
  });
});
