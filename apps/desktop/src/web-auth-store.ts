const TOKEN_KEY = 'previdenciar.web.idToken';
const REFRESH_KEY = 'previdenciar.web.refreshToken';
const EXPIRY_SKEW_SECONDS = 30;

type TokenPayload = { exp?: number };

function decodePayload(token: string): TokenPayload | null {
  try {
    const payload = token.split('.')[1];
    if (!payload) return null;
    const normalized = payload.replaceAll('-', '+').replaceAll('_', '/');
    const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, '=');
    return JSON.parse(atob(padded)) as TokenPayload;
  } catch {
    return null;
  }
}

export function isExpired(token: string): boolean {
  const payload = decodePayload(token);
  if (!payload?.exp) return false;
  return payload.exp <= Math.floor(Date.now() / 1000) + EXPIRY_SKEW_SECONDS;
}

export function storedToken(): string | null {
  const token = localStorage.getItem(TOKEN_KEY);
  return token && !isExpired(token) ? token : null;
}

export function refreshTokenValue(): string | null {
  return localStorage.getItem(REFRESH_KEY);
}

export function setSession(idToken: string, refreshToken?: string | null): void {
  localStorage.setItem(TOKEN_KEY, idToken);
  if (refreshToken) localStorage.setItem(REFRESH_KEY, refreshToken);
}

export function clearSession(): void {
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(REFRESH_KEY);
}

export async function refreshSession(): Promise<boolean> {
  const refreshToken = refreshTokenValue();
  if (!refreshToken) return false;
  const response = await fetch('/api/auth/refresh', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ refresh_token: refreshToken }),
  });
  if (!response.ok) {
    clearSession();
    return false;
  }
  const parsed = await response.json() as { id_token?: string; refresh_token?: string };
  if (!parsed.id_token) {
    clearSession();
    return false;
  }
  setSession(parsed.id_token, parsed.refresh_token ?? refreshToken);
  return true;
}

export async function validToken(): Promise<string | null> {
  const token = storedToken();
  if (token) return token;
  return (await refreshSession()) ? storedToken() : null;
}
