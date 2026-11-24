// Consentimento, contagem anônima de páginas e relato de erros do navegador (somente no site).
//
// A contagem só acontece depois do "Aceitar" e envia apenas o nome da tela (ex.: "clients").
// O relato de erro manda a mensagem técnica e a tela; o servidor ainda apaga e-mails e números longos.
// Nada disso usa cookie: a escolha fica no armazenamento local do navegador.

const CONSENT_KEY = 'previdenciar.consent';
export type Consent = 'granted' | 'denied' | null;

export function getConsent(): Consent {
  try {
    const value = localStorage.getItem(CONSENT_KEY);
    return value === 'granted' || value === 'denied' ? value : null;
  } catch {
    return null;
  }
}

export function setConsent(value: 'granted' | 'denied'): void {
  try {
    localStorage.setItem(CONSENT_KEY, value);
  } catch {
    /* navegador sem armazenamento: a escolha vale só nesta visita */
  }
  memory = value;
}

let memory: Consent = null;
let currentPage = 'desconhecida';
const counted = new Set<string>();

function consentGranted(): boolean {
  return (memory ?? getConsent()) === 'granted';
}

function post(path: string, body: unknown): void {
  try {
    void fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
      keepalive: true,
    }).catch(() => {});
  } catch {
    /* relato é opcional: nunca derruba a tela */
  }
}

/** Registra a tela atual. Conta no máximo uma vez por tela em cada carregamento. */
export function trackPage(page: string): void {
  if (!/^[a-z0-9-]{1,40}$/.test(page)) return;
  currentPage = page;
  if (!consentGranted() || counted.has(page)) return;
  counted.add(page);
  post('/api/metrics/pageview', { page });
}

let reported = 0;
let installed = false;

/** Envia ao servidor erros não tratados do navegador, no máximo 5 por carregamento. */
export function installErrorReporting(): void {
  if (installed || typeof window === 'undefined') return;
  installed = true;
  const report = (kind: string, message: string) => {
    if (reported >= 5 || !message) return;
    reported += 1;
    post('/api/client-errors', { kind, page: currentPage, message: message.slice(0, 500) });
  };
  window.addEventListener('error', event => report('erro', event.message || String(event.error ?? '')));
  window.addEventListener('unhandledrejection', event => {
    const reason = event.reason;
    report('promessa', reason instanceof Error ? reason.message : String(reason ?? ''));
  });
}
