import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { webApi } from '../src/web-api';

// Corpos e arquivos sintéticos. Nenhuma conta, sessão, CNIS ou PDF real.
// O objetivo é fixar o contrato da borda: sucesso só com JSON válido e falha
// sempre em português, sem erro de sintaxe, status numérico ou objeto cru.

type RespostaFalsa = { ok: boolean; status: number; text: () => Promise<string> };

function resposta(status: number, corpo: string): RespostaFalsa {
  return { ok: status >= 200 && status < 300, status, text: async () => corpo };
}

function responderCom(status: number, corpo: string) {
  const fetchFalso = vi.fn(async () => resposta(status, corpo) as unknown as Response);
  vi.stubGlobal('fetch', fetchFalso);
  return fetchFalso;
}

// O diálogo de arquivo usa um input criado em memória. Aqui ele recebe um
// arquivo sintético e dispara o evento, sem tocar no sistema de arquivos.
function simularEscolhaDeArquivo() {
  const criar = document.createElement.bind(document);
  vi.spyOn(document, 'createElement').mockImplementation(((tag: string) => {
    const elemento = criar(tag);
    if (tag === 'input') {
      queueMicrotask(() => {
        Object.defineProperty(elemento, 'files', {
          value: [new File(['%PDF sintetico'], 'sintetico.pdf', { type: 'application/pdf' })],
        });
        (elemento as HTMLInputElement).onchange?.(new Event('change'));
      });
    }
    return elemento;
  }) as typeof document.createElement);
}

const HTML_502 = '<!DOCTYPE html><html><body><h1>502 Bad Gateway</h1></body></html>';

async function mensagemDaFalha(executar: () => Promise<unknown>): Promise<string> {
  try {
    await executar();
  } catch (erro) {
    return erro instanceof Error ? erro.message : String(erro);
  }
  throw new Error('Esperava falha, mas a chamada concluiu.');
}

function exigirMensagemPropria(texto: string) {
  expect(texto).not.toMatch(/SyntaxError|Unexpected|JSON\.parse/i);
  expect(texto).not.toMatch(/\b\d{3}\b/);
  expect(texto).not.toMatch(/\[object Object\]/);
  expect(texto.length).toBeGreaterThan(0);
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('request por meio de boot', () => {
  it('aceita objeto JSON válido', async () => {
    responderCom(200, '{"workspace":{"clients":[]}}');
    await expect(webApi.boot()).resolves.toMatchObject({ workspace: { clients: [] } });
  });

  it('rejeita 200 com página HTML do SPA', async () => {
    responderCom(200, '<!doctype html><html><body>app</body></html>');
    const texto = await mensagemDaFalha(() => webApi.boot());
    expect(texto).toMatch(/operação falhou/i);
    exigirMensagemPropria(texto);
  });

  it('rejeita 200 com corpo vazio', async () => {
    responderCom(200, '');
    expect(await mensagemDaFalha(() => webApi.boot())).toMatch(/operação falhou/i);
  });

  it('rejeita 200 com o literal null', async () => {
    responderCom(200, 'null');
    expect(await mensagemDaFalha(() => webApi.boot())).toMatch(/operação falhou/i);
  });

  it('rejeita lista no endpoint que devolve registro', async () => {
    responderCom(200, '[{"id":1}]');
    expect(await mensagemDaFalha(() => webApi.boot())).toMatch(/operação falhou/i);
  });

  it('rejeita JSON truncado', async () => {
    responderCom(200, '{"workspace":');
    expect(await mensagemDaFalha(() => webApi.boot())).toMatch(/operação falhou/i);
  });
});

describe('request em endpoint que admite lista no topo', () => {
  it('preserva lista válida', async () => {
    responderCom(200, '[{"tipo":"sintetico"}]');
    await expect(webApi.tool('exemplo', { entrada: 1 })).resolves.toEqual([{ tipo: 'sintetico' }]);
  });

  it('continua rejeitando null e HTML', async () => {
    responderCom(200, 'null');
    expect(await mensagemDaFalha(() => webApi.tool('exemplo', {}))).toMatch(/operação falhou/i);
    responderCom(200, '<!doctype html><html></html>');
    expect(await mensagemDaFalha(() => webApi.tool('exemplo', {}))).toMatch(/operação falhou/i);
  });
});

describe('falha HTTP do request', () => {
  it('usa a mensagem do servidor em português', async () => {
    responderCom(401, '{"error":"Sessão ausente."}');
    const texto = await mensagemDaFalha(() => webApi.boot());
    expect(texto).toBe('Sessão ausente.');
  });

  it('não deixa código nem texto técnico aparecer em erro HTML', async () => {
    responderCom(502, HTML_502);
    const texto = await mensagemDaFalha(() => webApi.boot());
    expect(texto).toMatch(/indisponível|instantes/i);
    exigirMensagemPropria(texto);
  });

  it('não devolve objeto quando o campo error não é texto', async () => {
    responderCom(500, '{"error":{"code":"x"}}');
    const texto = await mensagemDaFalha(() => webApi.boot());
    expect(texto).toMatch(/operação falhou/i);
    exigirMensagemPropria(texto);
  });
});

describe('pickCnis', () => {
  it('aceita objeto válido', async () => {
    simularEscolhaDeArquivo();
    responderCom(200, '{"identity":{"full_name":null},"links":[],"warnings":[]}');
    const cnis = await webApi.pickCnis();
    expect(cnis).toMatchObject({ links: [], warnings: [] });
  });

  it('rejeita 200 com HTML', async () => {
    simularEscolhaDeArquivo();
    responderCom(200, '<!doctype html><html></html>');
    const texto = await mensagemDaFalha(() => webApi.pickCnis());
    expect(texto).toMatch(/importação do CNIS falhou/i);
    exigirMensagemPropria(texto);
  });

  it('rejeita 200 com null em vez de devolver null como CNIS', async () => {
    simularEscolhaDeArquivo();
    responderCom(200, 'null');
    const texto = await mensagemDaFalha(() => webApi.pickCnis());
    expect(texto).toMatch(/importação do CNIS falhou/i);
  });

  it('rejeita corpo vazio e lista', async () => {
    simularEscolhaDeArquivo();
    responderCom(200, '');
    expect(await mensagemDaFalha(() => webApi.pickCnis())).toMatch(/importação do CNIS falhou/i);
    responderCom(200, '[]');
    expect(await mensagemDaFalha(() => webApi.pickCnis())).toMatch(/importação do CNIS falhou/i);
  });

  it('traduz 502 HTML sem status nem texto técnico', async () => {
    simularEscolhaDeArquivo();
    responderCom(502, HTML_502);
    const texto = await mensagemDaFalha(() => webApi.pickCnis());
    expect(texto).toMatch(/indisponível|instantes/i);
    exigirMensagemPropria(texto);
  });
});

describe('pickCasePdfText', () => {
  it('aceita objeto válido', async () => {
    simularEscolhaDeArquivo();
    responderCom(200, '{"fileName":"sintetico.pdf","text":"x","usedOcr":false,"pages":1,"hashSha256":"00"}');
    await expect(webApi.pickCasePdfText()).resolves.toMatchObject({ pages: 1, usedOcr: false });
  });

  it('traduz 502 HTML sem lançar erro de sintaxe', async () => {
    simularEscolhaDeArquivo();
    responderCom(502, HTML_502);
    const texto = await mensagemDaFalha(() => webApi.pickCasePdfText());
    expect(texto).toMatch(/indisponível|instantes/i);
    exigirMensagemPropria(texto);
  });

  it('usa a mensagem do servidor quando o erro é texto', async () => {
    simularEscolhaDeArquivo();
    responderCom(400, '{"error":"O PDF está vazio."}');
    expect(await mensagemDaFalha(() => webApi.pickCasePdfText())).toBe('O PDF está vazio.');
  });

  it('não mostra objeto quando error não é texto', async () => {
    simularEscolhaDeArquivo();
    responderCom(400, '{"error":{"campo":"file"}}');
    const texto = await mensagemDaFalha(() => webApi.pickCasePdfText());
    expect(texto).toMatch(/leitura do PDF falhou/i);
    exigirMensagemPropria(texto);
  });

  it('rejeita 200 vazio ou null', async () => {
    simularEscolhaDeArquivo();
    responderCom(200, '');
    expect(await mensagemDaFalha(() => webApi.pickCasePdfText())).toMatch(/leitura do PDF falhou/i);
    responderCom(200, 'null');
    expect(await mensagemDaFalha(() => webApi.pickCasePdfText())).toMatch(/leitura do PDF falhou/i);
  });
});

describe('aiChat', () => {
  const pedido = {
    requestId: 'sintetico',
    providerId: 'plataforma',
    kind: 'openai',
    baseUrl: 'https://exemplo.invalid',
    model: 'modelo',
    messages: [{ role: 'user', content: 'pergunta sintética' }],
    consent: true,
    toolsEnabled: false,
  } as unknown as Parameters<typeof webApi.aiChat>[0];

  it('traduz 502 HTML', async () => {
    responderCom(502, HTML_502);
    const texto = await mensagemDaFalha(() => webApi.aiChat(pedido, () => {}));
    expect(texto).toMatch(/indisponível|instantes/i);
    exigirMensagemPropria(texto);
  });

  it('usa a mensagem do servidor quando o erro é texto', async () => {
    responderCom(503, '{"error":"O Copiloto está desligado nesta instalação."}');
    expect(await mensagemDaFalha(() => webApi.aiChat(pedido, () => {})))
      .toBe('O Copiloto está desligado nesta instalação.');
  });

  it('não transpõe objeto nem status para a mensagem', async () => {
    responderCom(500, '{"error":{"motivo":"falha"}}');
    const texto = await mensagemDaFalha(() => webApi.aiChat(pedido, () => {}));
    expect(texto).toMatch(/Copiloto não respondeu/i);
    exigirMensagemPropria(texto);
  });
});
