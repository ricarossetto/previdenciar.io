// Reparo de texto que chegou codificado duas vezes: o servidor grava UTF-8,
// algum trecho da cadeia lê esses bytes como Latin-1 e o navegador recebe
// "Sessao" com os acentos trocados por dois caracteres. O conserto fica na
// borda de exibição para que a mensagem do servidor não seja reescrita nem
// perdida. O literal do defeito não aparece aqui de propósito: o teste
// source-encoding reprova qualquer fonte que o contenha.
//
// A função só age quando há marcador de dupla codificação e a releitura
// produz UTF-8 válido; caso contrário devolve o texto original.

const MARCADOR = /[\u00c2\u00c3]/;

export function repairMojibake(texto: string): string {
  if (!texto || !MARCADOR.test(texto)) return texto;

  const bytes = new Uint8Array(texto.length);
  for (let i = 0; i < texto.length; i += 1) {
    const codigo = texto.charCodeAt(i);
    // Fora do Latin-1 não há reinterpretação possível.
    if (codigo > 0xff) return texto;
    bytes[i] = codigo;
  }

  try {
    const decodificado = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    // Só aceita se o resultado eliminou o marcador: evita "consertar"
    // texto que já estava certo.
    return MARCADOR.test(decodificado) ? texto : decodificado;
  } catch {
    return texto;
  }
}
