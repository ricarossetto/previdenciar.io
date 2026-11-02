import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

// Guarda de regressão: impede que a fonte volte a ser gravada com acentos
// codificados duas vezes (UTF-8 lido como Latin-1), que é o defeito que
// chegou à build publicada.
const RAIZES = [
  join(process.cwd(), 'src'),
  join(process.cwd(), '..', 'server', 'src'),
];
const EXTENSOES = new Set(['.ts', '.tsx', '.rs', '.css', '.html']);

function listar(diretorio: string): string[] {
  let itens: string[];
  try {
    itens = readdirSync(diretorio);
  } catch {
    return [];
  }
  const arquivos: string[] = [];
  for (const item of itens) {
    const caminho = join(diretorio, item);
    if (statSync(caminho).isDirectory()) arquivos.push(...listar(caminho));
    else if (EXTENSOES.has(caminho.slice(caminho.lastIndexOf('.')))) arquivos.push(caminho);
  }
  return arquivos;
}

// "Ã£" como bytes UTF-8 é C3 83 C2 A3: o marcador da dupla codificação.
const MARCADOR = Buffer.from([0xc3, 0x83, 0xc2]);

describe('codificação do código-fonte', () => {
  it('nenhum arquivo tem acento codificado duas vezes', () => {
    const suspeitos: string[] = [];
    for (const raiz of RAIZES) {
      for (const arquivo of listar(raiz)) {
        if (readFileSync(arquivo).includes(MARCADOR)) suspeitos.push(arquivo);
      }
    }
    expect(suspeitos).toEqual([]);
  });

  it('mensagens de erro do servidor decodificam como UTF-8 válido', () => {
    const arquivo = join(process.cwd(), '..', 'server', 'src', 'auth.rs');
    const texto = readFileSync(arquivo, 'utf8');
    expect(texto).toContain('Sessão ausente.');
  });
});
