// Leitura do retorno do provedor de identidade.
//
// O provedor devolve ?code=... quando dá certo e ?error=... quando o usuário
// cancela, a sessão expira ou o serviço falha. O texto do provedor nunca vai
// para a tela: ele é escolhido por terceiro e pode conter marcação. Traduzimos
// por uma lista fechada e devolvemos mensagem própria em qualquer outro caso.
//
// O state amarra o retorno à tentativa que o navegador iniciou. Ele é
// comparado antes de qualquer troca de código.

export type CallbackRead =
  | { kind: 'none' }
  | { kind: 'empty'; state: string | null }
  | { kind: 'code'; code: string; state: string | null }
  | { kind: 'error'; reason: string | null; state: string | null };

const MENSAGEM_PADRAO = 'Não foi possível concluir o login. Tente de novo.';

export function readCallback(pathname: string, search: URLSearchParams): CallbackRead {
  if (pathname !== '/auth/callback') return { kind: 'none' };

  const erro = search.get('error');
  const code = search.get('code');
  const state = search.get('state');

  // Erro tem precedência: um retorno com os dois não é um login válido.
  if (erro !== null) return { kind: 'error', reason: erro || null, state };

  const limpo = code?.trim();
  if (!limpo) return { kind: 'empty', state };

  return { kind: 'code', code: limpo, state };
}

export function describeCallbackError(reason: string | null): string {
  switch (reason) {
    case 'access_denied':
      return 'Você cancelou o acesso. Para entrar, tente de novo e autorize o acesso à sua conta.';
    case 'login_required':
      return 'Sua sessão de login expirou. Entre novamente para continuar.';
    case 'temporarily_unavailable':
      return 'O serviço de login está indisponível agora. Tente de novo em instantes.';
    case 'server_error':
      return 'O serviço de login falhou. Tente de novo em instantes.';
    case 'invalid_request':
      return 'O pedido de login foi recusado. Entre novamente.';
    default:
      return MENSAGEM_PADRAO;
  }
}

// Tentativa de login guardada no navegador: verifier do PKCE e state.
// Ler e apagar acontece juntos, para que o retorno só possa ser usado uma vez.
export type LoginAttempt = { verifier: string; state: string };

export function sameState(recebido: string | null, esperado: string | null): boolean {
  // Ausência de qualquer lado não comprova vínculo com esta tentativa.
  if (!recebido || !esperado) return false;
  // Comparação estrita e de tamanho fixo: o state é aleatório de 32 bytes em
  // base64url, então comprimentos diferentes já reprovam.
  if (recebido.length !== esperado.length) return false;
  let diferenca = 0;
  for (let i = 0; i < recebido.length; i += 1) {
    diferenca |= recebido.charCodeAt(i) ^ esperado.charCodeAt(i);
  }
  return diferenca === 0;
}

export type ConsumeOutcome =
  | { kind: 'nenhum'; limparUrl: false }
  | { kind: 'prosseguir'; code: string; verifier: string; limparUrl: true }
  | { kind: 'mensagem'; texto: string; limparUrl: boolean };

const MSG_TENTATIVA = 'A tentativa de login não confere. Entre novamente.';

// Consome a tentativa guardada. Sempre apaga, qualquer que seja o resultado,
// para que o mesmo retorno não volte a ser aceito num recarregamento.
// `storage` é recebido por parâmetro para permitir teste sem navegador.
export function consumeAttempt(storage: Pick<Storage, 'getItem' | 'removeItem'>, chave: string): LoginAttempt | null {
  const bruto = storage.getItem(chave);
  storage.removeItem(chave);
  if (!bruto) return null;
  try {
    const valor = JSON.parse(bruto) as { verifier?: unknown; state?: unknown };
    if (typeof valor?.verifier !== 'string' || typeof valor?.state !== 'string') return null;
    if (!valor.verifier || !valor.state) return null;
    return { verifier: valor.verifier, state: valor.state };
  } catch {
    return null;
  }
}

// Decide o destino do retorno e consome a tentativa uma única vez.
// A mensagem nunca reproduz o texto do provedor, o código nem o token.
export function resolveCallback(
  retorno: CallbackRead,
  consumir: (state: string | null) => LoginAttempt | null,
): ConsumeOutcome {
  if (retorno.kind === 'none') return { kind: 'nenhum', limparUrl: false };

  if (retorno.kind === 'error') {
    consumir(retorno.state);
    return { kind: 'mensagem', texto: describeCallbackError(retorno.reason), limparUrl: true };
  }

  if (retorno.kind === 'empty') {
    consumir(retorno.state);
    return {
      kind: 'mensagem',
      texto: 'O login não retornou um código válido. Entre novamente.',
      limparUrl: true,
    };
  }

  const tentativa = consumir(retorno.state);
  if (!tentativa || !sameState(retorno.state, tentativa.state)) {
    return { kind: 'mensagem', texto: MSG_TENTATIVA, limparUrl: true };
  }
  return { kind: 'prosseguir', code: retorno.code, verifier: tentativa.verifier, limparUrl: true };
}
