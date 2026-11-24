import { describe, expect, it } from 'vitest';
import { describeFailure, parseJsonSafe } from '../src/http-errors';

// Payloads sintéticos. Nenhum dado de cliente.
describe('parseJsonSafe', () => {
  it('devolve objeto quando o corpo é JSON', () => {
    expect(parseJsonSafe('{"error":"Sessão ausente."}')).toEqual({ error: 'Sessão ausente.' });
  });

  it('devolve null em corpo vazio', () => {
    expect(parseJsonSafe('')).toBeNull();
    expect(parseJsonSafe('   ')).toBeNull();
  });

  it('devolve null em página HTML de erro do proxy', () => {
    expect(parseJsonSafe('<!DOCTYPE html><html><body>502</body></html>')).toBeNull();
  });

  it('devolve null em texto truncado', () => {
    expect(parseJsonSafe('{"error":"corte')).toBeNull();
  });
});

describe('describeFailure', () => {
  it('usa a mensagem do servidor quando existe', () => {
    const texto = describeFailure(401, { error: 'Sessão ausente.' }, 'Falha.');
    expect(texto).toBe('Sessão ausente.');
  });

  it('recupera acentos corrompidos na mensagem do servidor', () => {
    const texto = describeFailure(401, { error: 'SessÃ£o ausente.' }, 'Falha.');
    expect(texto).toBe('Sessão ausente.');
  });

  it('não expõe código HTTP cru nem inglês quando não há corpo JSON', () => {
    const texto = describeFailure(500, null, 'Não foi possível concluir o login.');
    expect(texto).toBe('Não foi possível concluir o login.');
    expect(texto).not.toMatch(/\d{3}/);
    expect(texto).not.toMatch(/unexpected|json|token/i);
  });

  it('traduz página HTML de erro do proxy em 502', () => {
    const texto = describeFailure(502, null, 'Não foi possível concluir o login.');
    expect(texto).toMatch(/instantes/i);
    expect(texto).not.toMatch(/\d{3}/);
  });

  it('trata 429 com orientação de espera', () => {
    const texto = describeFailure(429, null, 'Falha.');
    expect(texto).toMatch(/aguarde/i);
  });

  it('trata 503 com orientação de nova tentativa', () => {
    const texto = describeFailure(503, null, 'Falha.');
    expect(texto).toMatch(/tente de novo|instantes/i);
  });

  it('ignora corpo JSON sem campo error', () => {
    expect(describeFailure(500, { ok: false }, 'Falha.')).toBe('Falha.');
  });

  it('ignora error que não é texto', () => {
    expect(describeFailure(500, { error: 42 }, 'Falha.')).toBe('Falha.');
  });
});
