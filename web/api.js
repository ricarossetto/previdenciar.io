(function () {
  'use strict';

  const STORAGE_BASE = 'pio-fabrica-base';
  const sameOrigin = location.protocol === 'http:' || location.protocol === 'https:';
  let base = '';
  try { base = localStorage.getItem(STORAGE_BASE) || ''; } catch (error) {}
  if (!base) base = sameOrigin ? '' : 'http://127.0.0.1:8787';
  base = base.replace(/\/+$/, '');

  class FabricaError extends Error {
    constructor(message, status, payload) {
      super(message);
      this.name = 'FabricaError';
      this.status = status;
      this.payload = payload;
    }
  }

  function queryString(options) {
    const params = new URLSearchParams();
    if (options && options.demonstrativo) params.set('demonstrativo', 'true');
    if (options && options.permalink) params.set('permalink', 'true');
    const text = params.toString();
    return text ? '?' + text : '';
  }

  async function request(path, options) {
    options = options || {};
    const url = base + path;
    const headers = Object.assign({ Accept: 'application/json' }, options.headers || {});
    let body = options.body;
    if (body && !(body instanceof FormData) && typeof body !== 'string') {
      headers['Content-Type'] = 'application/json';
      body = JSON.stringify(body);
    }
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), options.timeout || 200000);
    let response;
    let text = '';
    try {
      response = await fetch(url, {
        method: options.method || 'GET',
        headers,
        body,
        signal: controller.signal,
        mode: 'cors',
        cache: 'no-store'
      });
      text = await response.text();
    } catch (error) {
      const message = error && error.name === 'AbortError'
        ? 'Tempo esgotado ao falar com o servidor local.'
        : 'Não foi possível falar com o servidor local. Ele está rodando em ' + (base || location.origin) + '?';
      throw new FabricaError(message, 0, null);
    } finally {
      clearTimeout(timeout);
    }
    let payload = null;
    if (text) {
      try { payload = JSON.parse(text); } catch (error) { payload = null; }
    }
    if (!response.ok) {
      const messages = payload && Array.isArray(payload.messages) ? payload.messages : [];
      const message = messages.length
        ? messages.join(' · ')
        : (payload && payload.message) || ('A Fábrica respondeu HTTP ' + response.status + '.');
      throw new FabricaError(message, response.status, payload || { status: 'error', code: String(response.status), messages: [text.slice(0, 500)] });
    }
    if (payload == null) {
      throw new FabricaError('Resposta não JSON recebida do servidor local.', response.status, { text: text.slice(0, 1000) });
    }
    return payload;
  }

  const FabricaApi = {
    get base() { return base; },
    request: request,
    setBase(value) {
      base = String(value || '').trim().replace(/\/+$/, '');
      try { localStorage.setItem(STORAGE_BASE, base); } catch (error) {}
      return base;
    },
    health() { return request('/api/fabrica/health', { timeout: 8000 }); },
    fundamentos() { return request('/api/fabrica/fundamentos'); },
    marcosTemporais() { return request('/api/fabrica/marcos-temporais'); },
    calculadoras() { return request('/api/fabrica/calculadoras'); },
    tempo(dados, options) { return request('/api/fabrica/tempo' + queryString(options), { method: 'POST', body: dados }); },
    tempoPorGrupos(dados, options) { return request('/api/fabrica/tempo/por-grupos' + queryString(options), { method: 'POST', body: dados }); },
    rmi(dados, options) { return request('/api/fabrica/rmi' + queryString(options), { method: 'POST', body: dados }); },
    rmiPorFundamento(dados, options) { return request('/api/fabrica/rmi/por-fundamento' + queryString(options), { method: 'POST', body: dados }); },
    rmiDesvinculada(dados, options) { return request('/api/fabrica/rmi/desvinculada' + queryString(options), { method: 'POST', body: dados }); },
    atrasados(dados, options) { return request('/api/fabrica/atrasados' + queryString(options), { method: 'POST', body: dados }); },
    documento(tipo, file) {
      const form = new FormData();
      form.append('file', file, file.name || tipo + '.pdf');
      return request('/api/fabrica/documentos/' + tipo, { method: 'POST', body: form });
    },
    indices(nome, params) {
      const search = new URLSearchParams(params || {}).toString();
      return request('/api/fabrica/indices/' + nome + (search ? '?' + search : ''));
    },
    referencias: {
      bcb(serie, ultimos) {
        return request('/api/referencias/bcb/' + encodeURIComponent(serie) + '?ultimos=' + encodeURIComponent(ultimos || 12));
      },
      ibge(nome, meses) {
        return request('/api/referencias/ibge/' + encodeURIComponent(nome) + '?meses=' + encodeURIComponent(meses || 12));
      },
      async cjfImport(file) {
        const text = await file.text();
        return request('/api/referencias/cjf', {
          method: 'POST',
          body: text,
          headers: { 'Content-Type': 'text/csv; charset=utf-8' }
        });
      },
      cjfCached() {
        return request('/api/referencias/cjf');
      }
    },
    mcp(body) {
      return request('/api/fabrica/mcp', {
        method: 'POST',
        body,
        headers: { Accept: 'application/json, text/event-stream' },
        timeout: 180000
      });
    }
  };

  window.FabricaApi = FabricaApi;
  window.FabricaError = FabricaError;
})();
