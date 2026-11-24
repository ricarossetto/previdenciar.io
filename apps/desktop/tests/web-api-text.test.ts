import { describe, expect, it } from 'vitest';
import { repararJson, repararTexto } from '../src/web-api';

describe('normalizacao de texto da API', () => {
  it('recupera o mojibake do CNIS sem alterar portugues valido', () => {
    const quebrado = 'Aposentadoria por tempo de contribuiÃƒÂ§ÃƒÂ£o';
    expect(repararTexto(quebrado)).toBe('Aposentadoria por tempo de contribuição');
    expect(repararTexto('Carência apurada')).toBe('Carência apurada');
  });

  it('percorre objetos e listas inteiros', () => {
    const entrada = { titulo: 'Aposentadoria por tempo de contribuiÃƒÂ§ÃƒÂ£o', itens: ['CarÃƒÂªncia'] };
    expect(repararJson(entrada)).toEqual({ titulo: 'Aposentadoria por tempo de contribuição', itens: ['Carência'] });
  });
});