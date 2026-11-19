import { describe, expect, it } from 'vitest';
import { isExpired, shouldRefresh } from '../src/web-auth-store';

// Tokens sintéticos: só o payload importa. Nada de token real.
function tokenCom(payload: unknown): string {
  const corpo = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64url');
  return `cabecalho.${corpo}.assinatura`;
}

const AGORA = Math.floor(Date.now() / 1000);

describe('isExpired', () => {
  it('considera vencido um token com exp no passado', () => {
    expect(isExpired(tokenCom({ exp: AGORA - 3600 }))).toBe(true);
  });

  it('considera válido um token com exp no futuro', () => {
    expect(isExpired(tokenCom({ exp: AGORA + 3600 }))).toBe(false);
  });

  it('considera vencido quando falta exp', () => {
    expect(isExpired(tokenCom({ sub: 'x' }))).toBe(true);
  });

  it('considera vencido quando exp não é número', () => {
    expect(isExpired(tokenCom({ exp: '3600' }))).toBe(true);
    expect(isExpired(tokenCom({ exp: null }))).toBe(true);
  });

  it('considera vencido quando o payload não decodifica', () => {
    expect(isExpired('nao-e-um-jwt')).toBe(true);
    expect(isExpired('a.!!!.c')).toBe(true);
  });

  it('respeita a margem de segurança de 30 segundos', () => {
    expect(isExpired(tokenCom({ exp: AGORA + 10 }))).toBe(true);
    expect(isExpired(tokenCom({ exp: AGORA + 60 }))).toBe(false);
  });
});

describe('shouldRefresh', () => {
  it('pede renovação quando o token está vencido', () => {
    expect(shouldRefresh(tokenCom({ exp: AGORA - 10 }))).toBe(true);
  });

  it('não pede renovação quando o token está válido', () => {
    expect(shouldRefresh(tokenCom({ exp: AGORA + 3600 }))).toBe(false);
  });

  it('pede renovação quando não há token', () => {
    expect(shouldRefresh(null)).toBe(true);
  });
});
