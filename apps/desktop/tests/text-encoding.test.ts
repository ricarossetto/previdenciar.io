import { describe, expect, it } from 'vitest';
import { repairMojibake } from '../src/text-encoding';

// Textos sintéticos. Nenhum dado de cliente, CNIS ou caso real.
describe('repairMojibake', () => {
  it('recupera acentos quando o servidor envia UTF-8 lido como Latin-1', () => {
    expect(repairMojibake('SessÃ£o ausente.')).toBe('Sessão ausente.');
    expect(repairMojibake('SessÃ£o invÃ¡lida ou expirada.')).toBe('Sessão inválida ou expirada.');
    expect(repairMojibake('NÃ£o foi possÃ­vel concluir o login.')).toBe('Não foi possível concluir o login.');
  });

  it('preserva texto em português já correto', () => {
    const correto = 'Não foi possível concluir a operação.';
    expect(repairMojibake(correto)).toBe(correto);
  });

  it('preserva texto sem acento e texto vazio', () => {
    expect(repairMojibake('Sessao ausente.')).toBe('Sessao ausente.');
    expect(repairMojibake('')).toBe('');
  });

  it('preserva cifrão, porcentagem e moeda', () => {
    expect(repairMojibake('R$ 1.234,56 · 12,5%')).toBe('R$ 1.234,56 · 12,5%');
  });

  it('não altera sequência que não forma UTF-8 válido', () => {
    // "Ã©" isolado é dupla codificação válida; "Ã " (Ã seguido de espaço) não é.
    expect(repairMojibake('Ã ')).toBe('Ã ');
  });

  it('é idempotente', () => {
    const uma = repairMojibake('SessÃ£o ausente.');
    expect(repairMojibake(uma)).toBe(uma);
  });
});
