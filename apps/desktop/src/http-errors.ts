// Leitura tolerante de respostas HTTP e tradução da falha para texto que o
// advogado entende. O objetivo é não deixar vazar mensagem crua do proxy,
// erro de sintaxe de JSON ou código HTTP solto na tela do portão de entrada.
import { repairMojibake } from './text-encoding';

export function parseJsonSafe(texto: string): Record<string, unknown> | null {
  const limpo = texto.trim();
  if (!limpo) return null;
  // Página HTML de erro do proxy nunca é JSON: evita JSON.parse lançar.
  if (limpo.startsWith('<')) return null;
  try {
    const valor: unknown = JSON.parse(limpo);
    return valor && typeof valor === 'object' && !Array.isArray(valor)
      ? (valor as Record<string, unknown>)
      : null;
  } catch {
    return null;
  }
}

export function describeFailure(
  status: number,
  corpo: Record<string, unknown> | null,
  padrao: string,
): string {
  const bruto = corpo && typeof corpo.error === 'string' ? corpo.error.trim() : '';
  if (bruto) return repairMojibake(bruto);

  if (status === 429) {
    return 'Muitas tentativas seguidas. Aguarde alguns minutos e tente de novo.';
  }
  if (status === 503 || status === 502 || status === 504) {
    return 'O serviço está indisponível agora. Tente de novo em instantes.';
  }
  if (status === 401 || status === 403) {
    return 'Sua sessão expirou. Entre novamente.';
  }
  return padrao;
}
