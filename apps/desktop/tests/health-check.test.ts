import { describe, expect, it } from 'vitest';
import { readHealth } from '../src/health-check';

// Corpos sintéticos, sem dados de ambiente.
describe('readHealth', () => {
  it('aprova JSON do serviço com status ok', () => {
    const r = readHealth(200, 'application/json', '{"service":"previdenciar-web-server","status":"ok"}');
    expect(r.ok).toBe(true);
    expect(r.detail).toMatch(/ok/i);
  });

  it('reprova HTML 200 servido pelo SPA', () => {
    const r = readHealth(200, 'text/html', '<!doctype html><html><body>app</body></html>');
    expect(r.ok).toBe(false);
    expect(r.reason).toBe('not-json');
  });

  it('reprova JSON sem campo status', () => {
    const r = readHealth(200, 'application/json', '{"service":"x"}');
    expect(r.ok).toBe(false);
    expect(r.reason).toBe('missing-status');
  });

  it('reprova status diferente de ok', () => {
    const r = readHealth(200, 'application/json', '{"status":"degraded"}');
    expect(r.ok).toBe(false);
    expect(r.reason).toBe('status-not-ok');
  });

  it('reprova código HTTP de erro mesmo com corpo plausível', () => {
    const r = readHealth(503, 'application/json', '{"status":"ok"}');
    expect(r.ok).toBe(false);
    expect(r.reason).toBe('http-error');
  });

  it('reprova corpo vazio', () => {
    expect(readHealth(200, 'application/json', '').reason).toBe('empty');
  });

  it('reprova JSON malformado', () => {
    expect(readHealth(200, 'application/json', '{"status":').reason).toBe('not-json');
  });

  it('aceita content-type com charset', () => {
    const r = readHealth(200, 'application/json; charset=utf-8', '{"status":"ok"}');
    expect(r.ok).toBe(true);
  });
});
