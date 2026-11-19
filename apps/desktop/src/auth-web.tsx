import { useEffect, useRef, useState } from 'react';
import App from './App';
import { Button } from './components/ui';
import { setSession, validToken } from './web-auth-store';
import { describeFailure, parseJsonSafe } from './http-errors';
import { consumeAttempt, readCallback, resolveCallback } from './oauth-callback';

const ATTEMPT_KEY = 'previdenciar.web.oauthAttempt';
const CLIENT_ID = import.meta.env.VITE_COGNITO_CLIENT_ID ?? '4vla5ttd8j59jqj5oviviaugsj';
const DOMAIN = import.meta.env.VITE_COGNITO_DOMAIN ?? 'previdenciar-auth-557789254125.auth.sa-east-1.amazoncognito.com';
const GOOGLE_ENABLED = import.meta.env.VITE_COGNITO_GOOGLE === '1';

function base64Url(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replaceAll('=', '');
}

async function challenge(verifier: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier));
  return base64Url(new Uint8Array(digest));
}

async function beginAuthentication(provider?: 'Google', signup = false) {
  const verifier = base64Url(crypto.getRandomValues(new Uint8Array(48)));
  // O state amarra o retorno a esta tentativa. Ele é aleatório e separado do
  // verifier: se a origem devolver outro state, a troca nem chega a acontecer.
  const state = base64Url(crypto.getRandomValues(new Uint8Array(32)));
  sessionStorage.setItem(ATTEMPT_KEY, JSON.stringify({ verifier, state }));
  const params = new URLSearchParams({
    client_id: CLIENT_ID,
    response_type: 'code',
    scope: 'openid email profile',
    redirect_uri: `${window.location.origin}/auth/callback`,
    code_challenge_method: 'S256',
    code_challenge: await challenge(verifier),
    state,
  });
  if (provider) params.set('identity_provider', provider);
  window.location.assign(`https://${DOMAIN}/${signup ? 'signup' : 'oauth2/authorize'}?${params.toString()}`);
}

async function completeAuthentication(code: string, verifier: string) {
  const response = await fetch('/api/auth/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ code, code_verifier: verifier, redirect_uri: `${window.location.origin}/auth/callback` }),
  });
  const text = await response.text();
  const parsed = parseJsonSafe(text);
  const idToken = typeof parsed?.id_token === 'string' ? parsed.id_token : '';
  if (!response.ok || !idToken) {
    throw new Error(describeFailure(response.status, parsed, 'Não foi possível concluir o login.'));
  }
  setSession(idToken, typeof parsed?.refresh_token === 'string' ? parsed.refresh_token : null);
}

async function loginWithAtriumCredentials(username: string, password: string, code: string) {
  const response = await fetch('/api/auth/atrium-login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ username, password, code }),
  });
  const text = await response.text();
  const parsed = parseJsonSafe(text);
  const idToken = typeof parsed?.id_token === 'string' ? parsed.id_token : '';
  if (!response.ok || !idToken) {
    throw new Error(describeFailure(response.status, parsed, 'Não foi possível entrar com a conta do Atrium.'));
  }
  setSession(idToken, null);
}

async function sessionState(): Promise<'ready' | 'terms' | 'login'> {
  const token = await validToken();
  if (!token) return 'login';
  const response = await fetch('/api/me', { headers: { Authorization: `Bearer ${token}` } });
  if (response.status === 401) return 'login';
  const parsed = parseJsonSafe(await response.text());
  if (!response.ok) {
    throw new Error(describeFailure(response.status, parsed, 'Não foi possível carregar a sessão.'));
  }
  const data = (parsed ?? {}) as { terms?: { accepted?: boolean } };
  return data.terms?.accepted ? 'ready' : 'terms';
}

async function acceptTerms(): Promise<void> {
  const token = await validToken();
  if (!token) throw new Error('Sessão expirada. Entre novamente.');
  const response = await fetch('/api/terms/accept', {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: '{}',
  });
  if (!response.ok) throw new Error('Não foi possível registrar o aceite.');
}

function LoginPage({
  onLogin,
  onSignup,
  onAtriumLogin,
  error,
}: {
  onLogin: (provider?: 'Google') => void;
  onSignup: () => void;
  onAtriumLogin: (username: string, password: string, code: string) => Promise<void>;
  error: string | null;
}) {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [code, setCode] = useState('');
  const [submitting, setSubmitting] = useState(false);

  return <main className="loading-screen" style={{ padding: 24, paddingTop: 32, textAlign: 'left', maxWidth: 520, margin: '0 auto', height: 'auto', minHeight: '100dvh', placeContent: 'start center', overflowY: 'auto' }}>
    <div className="brand-name">Atrium · Previdenciário</div>
    <h1 style={{ marginTop: 20 }}>Entrar no escritório</h1>
    <p className="form-note">Use o mesmo usuário/e-mail e senha da sua assinatura do Atrium.</p>
    <form
      onSubmit={(event) => {
        event.preventDefault();
        if (!username.trim() || !password) return;
        setSubmitting(true);
        void onAtriumLogin(username.trim(), password, code.trim()).finally(() => setSubmitting(false));
      }}
      style={{ display: 'grid', gap: 12, width: '100%', marginTop: 12, marginBottom: 16 }}
    >
      <label style={{ display: 'grid', gap: 4 }}>
        <span className="form-note">Usuário ou e-mail do Atrium</span>
        <input
          type="text"
          autoComplete="username"
          value={username}
          onChange={(event) => setUsername(event.target.value)}
          placeholder="usuario ou email@escritorio.adv.br"
        />
      </label>
      <label style={{ display: 'grid', gap: 4 }}>
        <span className="form-note">Senha do Atrium</span>
        <input
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          placeholder="Sua senha do Atrium"
        />
      </label>
      <label style={{ display: 'grid', gap: 4 }}>
        <span className="form-note">Código 2FA do Atrium (se o 2FA estiver ativo na sua conta)</span>
        <input
          type="text"
          inputMode="numeric"
          autoComplete="one-time-code"
          value={code}
          onChange={(event) => setCode(event.target.value)}
          placeholder="Opcional (6 dígitos)"
        />
      </label>
      <Button primary type="submit" disabled={submitting || !username.trim() || !password}>
        {submitting ? 'Entrando com Atrium...' : 'Entrar com conta Atrium'}
      </Button>
    </form>
    {GOOGLE_ENABLED && <Button onClick={() => onLogin('Google')}>Continuar com Google</Button>}
    <Button onClick={() => onLogin()}>Entrar com e-mail e senha</Button>
    <Button onClick={onSignup}>Criar conta</Button>
    {error && <p role="alert" className="notice error">{error}</p>}
    <p className="form-note">Quem assina o Atrium (planos Pro e Pro IA) já tem acesso integrado com as mesmas credenciais.</p>
  </main>;
}

function TermsPage({ onAccept, error }: { onAccept: () => void; error: string | null }) {
  return <main className="loading-screen" style={{ padding: 24, paddingTop: 32, textAlign: 'left', maxWidth: 640, margin: '0 auto', height: 'auto', minHeight: '100dvh', placeContent: 'start center', overflowY: 'auto' }}>
    <div className="brand-name">previdenciar.io</div>
    <h1 style={{ marginTop: 20 }}>Antes de continuar</h1>
    <p className="form-note">Leia os termos de uso e a política de privacidade. O aceite fica registrado com a versão e a data.</p>
    <p><a href="/termos.html" target="_blank" rel="noreferrer">Termos de Uso</a> · <a href="/privacidade.html" target="_blank" rel="noreferrer">Política de Privacidade</a></p>
    <Button primary onClick={onAccept}>Li e aceito os termos</Button>
    {error && <p role="alert" className="notice error">{error}</p>}
  </main>;
}

export function WebAuthGate() {
  const [state, setState] = useState<'checking' | 'login' | 'terms' | 'ready'>('checking');
  const [error, setError] = useState<string | null>(null);
  // O efeito roda duas vezes em desenvolvimento. A primeira passagem já
  // consumiu a tentativa e limpou a URL; a segunda não pode tratar o retorno
  // como divergente nem competir com a troca de código em andamento.
  const iniciado = useRef(false);

  useEffect(() => {
    if (import.meta.env.VITE_AUTH_DISABLED === '1') {
      setState('ready');
      return;
    }
    if (iniciado.current) return;
    iniciado.current = true;
    const url = new URL(window.location.href);
    const hashParams = new URLSearchParams(url.hash.startsWith('#') ? url.hash.slice(1) : '');
    const atriumSsoToken = hashParams.get('atrium_sso') ?? url.searchParams.get('atrium_sso');
    if (atriumSsoToken) {
      window.history.replaceState({}, '', '/');
      setSession(atriumSsoToken, null);
      void sessionState()
        .then(setState)
        .catch((reason) => {
          setError(reason instanceof Error ? reason.message : 'Falha ao validar sessão do Atrium.');
          setState('login');
        });
      return;
    }
    const retorno = readCallback(url.pathname, url.searchParams);
    // A decisão e o consumo da tentativa acontecem antes de qualquer rede.
    // Limpar a URL na hora evita que um recarregamento reenvie o mesmo código;
    // o consumo único dentro de consumeAttempt garante o mesmo para efeitos
    // duplicados do StrictMode em desenvolvimento.
    const decisao = resolveCallback(retorno, () => consumeAttempt(sessionStorage, ATTEMPT_KEY));
    if (decisao.limparUrl) window.history.replaceState({}, '', '/');
    if (decisao.kind === 'prosseguir') {
      void completeAuthentication(decisao.code, decisao.verifier)
        .then(sessionState)
        .then(setState)
        .catch((reason) => {
          setError(reason instanceof Error ? reason.message : 'Falha ao entrar.');
          setState('login');
        });
      return;
    }
    if (decisao.kind === 'mensagem') {
      // Mensagem fechada: nunca reproduz o texto do provedor, o code ou o token.
      setError(decisao.texto);
      setState('login');
      return;
    }
    void sessionState().then(setState).catch((reason) => {
      setError(reason instanceof Error ? reason.message : 'Falha ao carregar a sessão.');
      setState('login');
    });
  }, []);

  if (state === 'checking') return <div className="loading-screen" role="status">Verificando sessão...</div>;
  if (state === 'login') return (
    <LoginPage
      error={error}
      onLogin={(provider) => void beginAuthentication(provider)}
      onSignup={() => void beginAuthentication(undefined, true)}
      onAtriumLogin={async (username, password, code) => {
        setError(null);
        try {
          await loginWithAtriumCredentials(username, password, code);
          const next = await sessionState();
          setState(next);
        } catch (reason) {
          setError(reason instanceof Error ? reason.message : 'Falha ao entrar com o Atrium.');
        }
      }}
    />
  );
  if (state === 'terms') return <TermsPage error={error} onAccept={() => void acceptTerms().then(() => setState('ready')).catch((reason) => setError(reason instanceof Error ? reason.message : 'Falha ao aceitar.'))} />;
  return <App />;
}
