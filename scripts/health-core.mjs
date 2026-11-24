// Núcleo único da conferência de saúde da API.
//
// O mesmo arquivo serve ao aplicativo (apps/desktop/src/health-check.ts) e ao
// script de linha de comando (scripts/check-health.mjs). É JavaScript puro,
// sem dependências: roda no Node declarado pelo repositório sem exigir
// suporte a TypeScript embutido.
//
// O servidor publica o SPA em qualquer rota não-API. Por isso /health pode
// responder 200 com HTML e parecer saudável. A checagem válida é /api/health
// com corpo JSON e status "ok".

/**
 * @typedef {'ok' | 'http-error' | 'empty' | 'not-json' | 'missing-status' | 'status-not-ok'} HealthReason
 */

/**
 * @typedef {object} HealthResult
 * @property {boolean} ok
 * @property {HealthReason} reason
 * @property {string} detail
 */

/**
 * @param {string | null | undefined} contentType
 * @param {string | null | undefined} body
 * @returns {boolean}
 */
export function pareceJson(contentType, body) {
  const tipo = String(contentType || '').toLowerCase();
  if (tipo.includes('json')) return true;
  const inicio = String(body || '').trim().slice(0, 1);
  return inicio === '{' || inicio === '[';
}

/**
 * @param {number} status
 * @param {string | null | undefined} contentType
 * @param {string | null | undefined} body
 * @returns {HealthResult}
 */
export function readHealth(status, contentType, body) {
  const texto = String(body || '');

  if (!Number.isFinite(status) || status < 200 || status >= 300) {
    return { ok: false, reason: 'http-error', detail: `A API respondeu ${status}.` };
  }
  if (!texto.trim()) {
    return { ok: false, reason: 'empty', detail: 'A API respondeu sem corpo.' };
  }
  if (!pareceJson(contentType, texto)) {
    return { ok: false, reason: 'not-json', detail: 'A resposta não é JSON: provavelmente a página do aplicativo.' };
  }

  /** @type {Record<string, unknown>} */
  let corpo;
  try {
    const valor = JSON.parse(texto);
    if (!valor || typeof valor !== 'object' || Array.isArray(valor)) {
      return { ok: false, reason: 'not-json', detail: 'A resposta não é um objeto JSON.' };
    }
    corpo = valor;
  } catch {
    return { ok: false, reason: 'not-json', detail: 'A resposta não é JSON válido.' };
  }

  if (!('status' in corpo)) {
    return { ok: false, reason: 'missing-status', detail: 'A resposta não informa o campo status.' };
  }
  if (corpo.status !== 'ok') {
    return { ok: false, reason: 'status-not-ok', detail: `O serviço informou status ${String(corpo.status)}.` };
  }

  const servico = typeof corpo.service === 'string' ? corpo.service : 'serviço';
  const versao = typeof corpo.version === 'string' ? ` ${corpo.version}` : '';
  return { ok: true, reason: 'ok', detail: `${servico}${versao} respondeu ok.` };
}
