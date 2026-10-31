import { useEffect, useState } from 'react';
import App from './App';
import { Button } from './components/ui';
import { setSession, validToken } from './web-auth-store';

const VERIFIER_KEY = 'previdenciar.web.codeVerifier';
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
  sessionStorage.setItem(VERIFIER_KEY, verifier);
  const params = new URLSearchParams({
    client_id: CLIENT_ID,
    response_type: 'code',
    scope: 'openid email profile',
    redirect_uri: `${window.location.origin}/auth/callback`,
    code_challenge_method: 'S256',
    code_challenge: await challenge(verifier),
  });
  if (provider) params.set('identity_provider', provider);
  window.location.assign(`https://${DOMAIN}/${signup ? 'signup' : 'oauth2/authorize'}?${params.toString()}`);
}

async function completeAuthentication(code: string) {
  const verifier = sessionStorage.getItem(VERIFIER_KEY);
  if (!verifier) throw new Error('Sessão de login expirada. Tente novamente.');
  const response = await fetch('/api/auth/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ code, code_verifier: verifier, redirect_uri: `${window.location.origin}/auth/callback` }),
  });
  const text = await response.text();
  const parsed = text ? JSON.parse(text) : null;
  if (!response.ok || !parsed?.id_token) throw new Error(parsed?.error ?? 'Não foi possível concluir o login.');
  setSession(parsed.id_token, parsed.refresh_token);
  sessionStorage.removeItem(VERIFIER_KEY);
  window.history.replaceState({}, '', '/');
}

async function sessionState(): Promise<'ready' | 'terms' | 'login'> {
  const token = await validToken();
  if (!token) return 'login';
  const response = await fetch('/api/me', { headers: { Authorization: `Bearer ${token}` } });
  if (response.status === 401) return 'login';
  if (!response.ok) throw new Error('Não foi possível carregar a sessão.');
  const data = await response.json() as { terms?: { accepted?: boolean } };
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

function LoginPage({ onLogin, onSignup, error }: { onLogin: (provider?: 'Google') => void; onSignup: () => void; error: string | null }) {
  return <main className="loading-screen" style={{ padding: 24, paddingTop: 32, textAlign: 'left', maxWidth: 520, margin: '0 auto', height: 'auto', minHeight: '100dvh', placeContent: 'start center', overflowY: 'auto' }}>
    <div className="brand-name">previdenciar.io</div>
    <h1 style={{ marginTop: 20 }}>Entrar no escritório</h1>
    <p className="form-note">Use sua conta autorizada para abrir a base previdenciária.</p>
    {GOOGLE_ENABLED && <Button primary onClick={() => onLogin('Google')}>Continuar com Google</Button>}
    <Button primary={!GOOGLE_ENABLED} onClick={() => onLogin()}>Entrar com e-mail e senha</Button>
    <Button onClick={onSignup}>Criar conta</Button>
    {error && <p role="alert" className="notice error">{error}</p>}
    <p className="form-note">O cadastro é aberto com confirmação de e-mail. O módulo avançado continua sujeito à permissão do piloto.</p>
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

  useEffect(() => {
    if (import.meta.env.VITE_AUTH_DISABLED === '1') {
      setState('ready');
      return;
    }
    const url = new URL(window.location.href);
    const code = url.searchParams.get('code');
    if (url.pathname === '/auth/callback' && code) {
      void completeAuthentication(code)
        .then(sessionState)
        .then(setState)
        .catch((reason) => {
          setError(reason instanceof Error ? reason.message : 'Falha ao entrar.');
          setState('login');
        });
      return;
    }
    void sessionState().then(setState).catch((reason) => {
      setError(reason instanceof Error ? reason.message : 'Falha ao carregar a sessão.');
      setState('login');
    });
  }, []);

  if (state === 'checking') return <div className="loading-screen" role="status">Verificando sessão...</div>;
  if (state === 'login') return <LoginPage error={error} onLogin={(provider) => void beginAuthentication(provider)} onSignup={() => void beginAuthentication(undefined, true)} />;
  if (state === 'terms') return <TermsPage error={error} onAccept={() => void acceptTerms().then(() => setState('ready')).catch((reason) => setError(reason instanceof Error ? reason.message : 'Falha ao aceitar.'))} />;
  return <App />;
}
