'use strict';

const http = require('node:http');
const https = require('node:https');
const fs = require('node:fs');
const path = require('node:path');
const { URL } = require('node:url');

const ROOT = path.resolve(__dirname, '..');
const WEB = path.join(ROOT, 'web');

function loadEnvFile(file) {
  let text;
  try {
    text = fs.readFileSync(file, 'utf8');
  } catch (error) {
    return;
  }
  for (const line of text.split(/\r?\n/)) {
    const match = line.match(/^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$/);
    if (!match) continue;
    let value = match[2];
    if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1);
    }
    if (!(match[1] in process.env)) process.env[match[1]] = value;
  }
}

loadEnvFile(path.join(ROOT, '.env'));
loadEnvFile(path.join(__dirname, '.env'));

const HOST = process.env.HOST || '127.0.0.1';
const PORT = Number(process.env.PORT || 8787);
const UPSTREAM = String(process.env.FABRICA_BASE_URL || 'https://fabrica-de-calculos.app.trf3.jus.br').replace(/\/+$/, '');
const API_KEY = process.env.FABRICA_API_KEY || '';
const MAX_JSON_BYTES = 12 * 1024 * 1024;
const MAX_UPLOAD_BYTES = 32 * 1024 * 1024;
const MAX_UPSTREAM_BYTES = 48 * 1024 * 1024;
const REF_CACHE_MS = 6 * 60 * 60 * 1000;
const REF_CACHE = new Map();
const IBGE_SERIES = {
  ipca: { agregado: '7060', variavel: '63', nome: 'IPCA — variação mensal' },
  inpc: { agregado: '7063', variavel: '44', nome: 'INPC — variação mensal' }
};
const CJF_TABLE = { headers: [], rows: [], uploadedAt: null, source: null };

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.webp': 'image/webp',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.ico': 'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.pdf': 'application/pdf'
};

const ROUTES = [
  { method: 'POST', path: '/api/fabrica/tempo', target: '/rgps/api/v1/calculos-tempo', timeout: 150000 },
  { method: 'POST', path: '/api/fabrica/tempo/por-grupos', target: '/rgps/api/v1/calculos-tempo/por-grupos', timeout: 150000 },
  { method: 'POST', path: '/api/fabrica/tempo/prevjud', target: '/rgps/api/v1/calculos-tempo/prevjud/calcular', timeout: 150000 },
  { method: 'POST', path: '/api/fabrica/tempo/prevjud/por-grupos', target: '/rgps/api/v1/calculos-tempo/prevjud/calcular-por-grupos', timeout: 150000 },
  { method: 'POST', path: '/api/fabrica/tempo/calc/ler', target: '/rgps/api/v1/calculos-tempo/arquivo-calc/ler', kind: 'multipart', timeout: 150000 },
  { method: 'POST', path: '/api/fabrica/tempo/calc/calcular', target: '/rgps/api/v1/calculos-tempo/arquivo-calc/calcular', kind: 'multipart', timeout: 150000 },
  { method: 'POST', path: '/api/fabrica/tempo/calc/calcular-por-grupos', target: '/rgps/api/v1/calculos-tempo/arquivo-calc/calcular-por-grupos', kind: 'multipart', timeout: 150000 },
  { method: 'POST', path: '/api/fabrica/atrasados', target: '/rgps/api/v1/calculos-atrasados', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/atrasados/calc/ler', target: '/rgps/api/v1/calculos-atrasados/arquivo-calc/ler', kind: 'multipart', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/atrasados/calc/calcular', target: '/rgps/api/v1/calculos-atrasados/arquivo-calc/calcular', kind: 'multipart', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi', target: '/rgps/api/v1/calculos-rmi', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi/por-fundamento', target: '/rgps/api/v1/calculos-rmi/por-fundamento', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi/desvinculada', target: '/rgps/api/v1/calculos-rmi/desvinculada', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi/async', target: '/rgps/api/v1/calculos-rmi/async', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi/stream', target: '/rgps/api/v1/calculos-rmi/stream', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi/calc/ler', target: '/rgps/api/v1/calculos-rmi/arquivo-calc/ler', kind: 'multipart', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi/calc/ler-desvinculada', target: '/rgps/api/v1/calculos-rmi/arquivo-calc/ler-desvinculada', kind: 'multipart', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi/calc/calcular', target: '/rgps/api/v1/calculos-rmi/arquivo-calc/calcular', kind: 'multipart', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi/calc/calcular-desvinculada', target: '/rgps/api/v1/calculos-rmi/arquivo-calc/calcular-desvinculada', kind: 'multipart', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi/calc/calcular-async', target: '/rgps/api/v1/calculos-rmi/arquivo-calc/calcular/async', kind: 'multipart', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/rmi/calc/calcular-stream', target: '/rgps/api/v1/calculos-rmi/arquivo-calc/calcular/stream', kind: 'multipart', timeout: 180000 },
  { method: 'POST', path: '/api/fabrica/exchange', target: '/apps/api/v1/exchange', kind: 'multipart', timeout: 180000 }
];

const DOCUMENT_TYPES = new Set(['cnis', 'rdctc', 'carta', 'pap', 'prevjud']);
const INDEX_TYPES = new Set([
  'tabela',
  'reajuste',
  'moedas',
  'tabua-mortalidade',
  'adiantamento',
  'limites',
  'limite-baixa-renda-reclusao',
  'os121',
  'reajuste-pre-real'
]);

function cors(req, res) {
  const origin = req.headers.origin;
  const allowed = !origin || origin === 'null' || /^https?:\/\/(localhost|127\.0\.0\.1)(:\d+)?$/.test(origin);
  res.setHeader('Vary', 'Origin');
  if (allowed) {
    res.setHeader('Access-Control-Allow-Origin', origin || '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Accept, X-Requested-With');
    res.setHeader('Access-Control-Max-Age', '600');
  }
}

function securityHeaders(res) {
  res.setHeader('X-Content-Type-Options', 'nosniff');
  res.setHeader('Referrer-Policy', 'no-referrer');
}

function sendJson(req, res, status, body, extraHeaders) {
  cors(req, res);
  securityHeaders(res);
  const payload = Buffer.from(JSON.stringify(body), 'utf8');
  res.statusCode = status;
  res.setHeader('Content-Type', 'application/json; charset=utf-8');
  res.setHeader('Content-Length', payload.length);
  res.setHeader('Cache-Control', 'no-store');
  if (extraHeaders) for (const [key, value] of Object.entries(extraHeaders)) res.setHeader(key, value);
  res.end(payload);
}

function refCacheGet(key) {
  const item = REF_CACHE.get(key);
  if (!item) return null;
  if (item.expires <= Date.now()) {
    REF_CACHE.delete(key);
    return null;
  }
  return item;
}

function refCacheSet(key, value) {
  if (REF_CACHE.size >= 200) {
    const oldest = REF_CACHE.keys().next().value;
    if (oldest) REF_CACHE.delete(oldest);
  }
  REF_CACHE.set(key, { value, expires: Date.now() + REF_CACHE_MS });
  return value;
}

async function fetchJson(url, timeout) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeout || 20000);
  let response;
  try {
    response = await fetch(url, {
      headers: {
        accept: 'application/json',
        'user-agent': 'previdenciar.io (referencias publicas)'
      },
      signal: controller.signal
    });
  } catch (error) {
    if (error && error.name === 'AbortError') throw Object.assign(new Error('Tempo esgotado ao consultar a fonte pública.'), { statusCode: 504 });
    throw Object.assign(new Error('Falha de rede ao consultar a fonte pública: ' + error.message), { statusCode: 502 });
  } finally {
    clearTimeout(timer);
  }
  if (!response.ok) throw Object.assign(new Error('A fonte pública respondeu HTTP ' + response.status + '.'), { statusCode: 502 });
  try {
    return await response.json();
  } catch (error) {
    throw Object.assign(new Error('A fonte pública não devolveu JSON válido.'), { statusCode: 502 });
  }
}

function competenciaRange(meses) {
  const end = new Date();
  end.setDate(1);
  const start = new Date(end.getFullYear(), end.getMonth() - (meses - 1), 1);
  const fmt = (date) => String(date.getFullYear()) + String(date.getMonth() + 1).padStart(2, '0');
  return fmt(start) + '-' + fmt(end);
}

function parseCsv(text) {
  const clean = String(text || '').replace(/^\uFEFF/, '');
  const firstLine = clean.split(/\r?\n/)[0] || '';
  const counts = { ';': 0, ',': 0, '\t': 0 };
  for (const char of firstLine) if (char in counts) counts[char]++;
  let delimiter = ';';
  if (counts[','] > counts[delimiter]) delimiter = ',';
  if (counts['\t'] > counts[delimiter]) delimiter = '\t';
  const rows = [];
  let row = [];
  let field = '';
  let quoted = false;
  for (let index = 0; index < clean.length; index++) {
    const char = clean[index];
    if (quoted) {
      if (char === '"') {
        if (clean[index + 1] === '"') {
          field += '"';
          index++;
        } else {
          quoted = false;
        }
      } else {
        field += char;
      }
      continue;
    }
    if (char === '"') {
      quoted = true;
    } else if (char === delimiter) {
      row.push(field.trim());
      field = '';
    } else if (char === '\n') {
      row.push(field.trim());
      if (row.some((cell) => cell !== '')) rows.push(row);
      row = [];
      field = '';
    } else if (char !== '\r') {
      field += char;
    }
  }
  row.push(field.trim());
  if (row.some((cell) => cell !== '')) rows.push(row);
  if (!rows.length) return { headers: [], rows: [] };
  const headers = rows[0].map((header, index) => header || 'Coluna ' + (index + 1));
  return { headers, rows: rows.slice(1, 5001) };
}

async function handleReferencias(req, res, parsed) {
  const pathname = parsed.pathname.replace(/\/+$/, '') || '/';

  if (req.method === 'POST' && pathname === '/api/referencias/cjf') {
    const body = await readBody(req, 8 * 1024 * 1024);
    const table = parseCsv(body.toString('utf8'));
    if (!table.headers.length) {
      sendJson(req, res, 400, { status: 'error', code: '400', messages: ['CSV vazio ou sem cabeçalho.'], result: null });
      return;
    }
    CJF_TABLE.headers = table.headers;
    CJF_TABLE.rows = table.rows;
    CJF_TABLE.uploadedAt = new Date().toISOString();
    CJF_TABLE.source = 'CJF SICOM (arquivo CSV importado)';
    sendJson(req, res, 200, {
      status: 'ok',
      code: '200',
      messages: [],
      source: CJF_TABLE.source,
      cached: false,
      result: { headers: CJF_TABLE.headers, rows: CJF_TABLE.rows, count: CJF_TABLE.rows.length, uploadedAt: CJF_TABLE.uploadedAt }
    });
    return;
  }

  if (req.method === 'GET' && pathname === '/api/referencias/cjf') {
    if (!CJF_TABLE.headers.length) {
      sendJson(req, res, 200, {
        status: 'ok',
        code: '200',
        messages: [],
        source: 'CJF SICOM (arquivo CSV importado)',
        cached: true,
        empty: true,
        result: { headers: [], rows: [], count: 0, uploadedAt: null }
      });
      return;
    }
    sendJson(req, res, 200, {
      status: 'ok',
      code: '200',
      messages: [],
      source: CJF_TABLE.source,
      cached: true,
      result: { headers: CJF_TABLE.headers, rows: CJF_TABLE.rows, count: CJF_TABLE.rows.length, uploadedAt: CJF_TABLE.uploadedAt }
    });
    return;
  }

  const bcbMatch = pathname.match(/^\/api\/referencias\/bcb\/(\d{1,5})$/);
  if (req.method === 'GET' && bcbMatch) {
    const serie = bcbMatch[1];
    const query = new URLSearchParams(parsed.search);
    const ultimos = Math.min(Math.max(Number(query.get('ultimos') || 12), 1), 120);
    let url;
    if (query.get('dataInicial') && query.get('dataFinal')) {
      const start = query.get('dataInicial');
      const end = query.get('dataFinal');
      if (!/^\d{2}\/\d{2}\/\d{4}$/.test(start) || !/^\d{2}\/\d{2}\/\d{4}$/.test(end)) {
        sendJson(req, res, 400, { status: 'error', code: '400', messages: ['Datas devem usar DD/MM/AAAA.'], result: null });
        return;
      }
      url = 'https://api.bcb.gov.br/dados/serie/bcdata.sgs.' + serie + '/dados?formato=json&dataInicial=' + encodeURIComponent(start) + '&dataFinal=' + encodeURIComponent(end);
    } else {
      url = 'https://api.bcb.gov.br/dados/serie/bcdata.sgs.' + serie + '/dados/ultimos/' + ultimos + '?formato=json';
    }
    const key = 'bcb:' + url;
    const cached = refCacheGet(key);
    if (cached) {
      res.setHeader('X-Ref-Cache', 'HIT');
      sendJson(req, res, 200, { status: 'ok', code: '200', messages: [], source: 'Banco Central do Brasil — SGS', cached: true, result: cached.value });
      return;
    }
    const raw = await fetchJson(url, 20000);
    const series = Array.isArray(raw) ? raw.map((item) => ({ data: item.data, valor: Number(String(item.valor).replace(',', '.')) })) : [];
    const value = { serie, series };
    refCacheSet(key, value);
    res.setHeader('X-Ref-Cache', 'MISS');
    sendJson(req, res, 200, { status: 'ok', code: '200', messages: [], source: 'Banco Central do Brasil — SGS', cached: false, result: value });
    return;
  }

  const ibgeMatch = pathname.match(/^\/api\/referencias\/ibge\/(ipca|inpc)$/);
  if (req.method === 'GET' && ibgeMatch) {
    const name = ibgeMatch[1];
    const meta = IBGE_SERIES[name];
    const query = new URLSearchParams(parsed.search);
    const meses = Math.min(Math.max(Number(query.get('meses') || 12), 1), 120);
    let periodos = query.get('periodos') || '';
    if (periodos && !/^\d{4,6}(-\d{4,6})?$/.test(periodos)) {
      sendJson(req, res, 400, { status: 'error', code: '400', messages: ['Períodos devem usar AAAAMM ou AAAAMM-AAAAMM.'], result: null });
      return;
    }
    if (!periodos) periodos = competenciaRange(meses);
    const url = 'https://servicodados.ibge.gov.br/api/v3/agregados/' + meta.agregado + '/periodos/' + periodos + '/variaveis/' + meta.variavel + '?localidades=N1[all]';
    const key = 'ibge:' + url;
    const cached = refCacheGet(key);
    if (cached) {
      res.setHeader('X-Ref-Cache', 'HIT');
      sendJson(req, res, 200, { status: 'ok', code: '200', messages: [], source: 'IBGE — SIDRA', cached: true, result: cached.value });
      return;
    }
    const raw = await fetchJson(url, 20000);
    const first = Array.isArray(raw) ? raw[0] : raw;
    const serie = first && first.resultados && first.resultados[0] && first.resultados[0].series && first.resultados[0].series[0] && first.resultados[0].series[0].serie;
    const series = serie ? Object.entries(serie).map(([periodo, valor]) => ({ periodo: periodo.slice(0, 4) + '-' + periodo.slice(4), valor: Number(valor) })) : [];
    const value = { serie: name, nome: meta.nome, series };
    refCacheSet(key, value);
    res.setHeader('X-Ref-Cache', 'MISS');
    sendJson(req, res, 200, { status: 'ok', code: '200', messages: [], source: 'IBGE — SIDRA', cached: false, result: value });
    return;
  }

  sendJson(req, res, 404, { status: 'error', code: '404', messages: ['Rota de referência não encontrada.'], result: null });
}

function readBody(req, limit) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    let settled = false;
    const fail = (error) => {
      if (settled) return;
      settled = true;
      reject(error);
    };
    req.on('data', (chunk) => {
      if (settled) return;
      size += chunk.length;
      if (size > limit) {
        fail(Object.assign(new Error('Corpo da requisição excede o limite local.'), { statusCode: 413 }));
        return;
      }
      chunks.push(chunk);
    });
    req.on('end', () => {
      if (settled) return;
      settled = true;
      resolve(Buffer.concat(chunks));
    });
    req.on('error', fail);
  });
}

function upstreamHeaders(accept) {
  return {
    accept: accept || 'application/json, text/html;q=0.9, */*;q=0.8',
    'accept-language': 'pt-BR,pt;q=0.9',
    'accept-encoding': 'identity',
    connection: 'close',
    'user-agent': 'Mozilla/5.0 (previdenciar.io; calculo previdenciario)',
    'x-api-key': API_KEY
  };
}

function requestUpstream(options) {
  return new Promise((resolve, reject) => {
    let url;
    try {
      url = new URL(UPSTREAM + options.target + options.search);
    } catch (error) {
      reject(Object.assign(new Error('URL da Fábrica inválida.'), { statusCode: 502 }));
      return;
    }
    const lib = url.protocol === 'https:' ? https : http;
    const headers = upstreamHeaders(options.accept);
    if (options.contentType) headers['content-type'] = options.contentType;
    if (options.body && options.body.length) headers['content-length'] = String(options.body.length);
    const request = lib.request(url, { method: options.method, headers }, (response) => {
      const chunks = [];
      let size = 0;
      let finished = false;
      const fail = (error) => {
        if (finished) return;
        finished = true;
        reject(error);
      };
      response.on('data', (chunk) => {
        if (finished) return;
        size += chunk.length;
        if (size > MAX_UPSTREAM_BYTES) {
          fail(Object.assign(new Error('Resposta da Fábrica excede o limite local.'), { statusCode: 502 }));
          response.destroy();
          return;
        }
        chunks.push(chunk);
      });
      response.on('end', () => {
        if (finished) return;
        finished = true;
        resolve({
          status: response.statusCode || 502,
          headers: response.headers,
          body: Buffer.concat(chunks)
        });
      });
      response.on('error', fail);
      response.on('aborted', () => fail(Object.assign(new Error('A Fábrica interrompeu a resposta.'), { statusCode: 502 })));
    });
    request.setTimeout(options.timeout || 90000, () => {
      request.destroy(Object.assign(new Error('Tempo esgotado ao consultar a Fábrica.'), { statusCode: 504 }));
    });
    request.on('error', (error) => {
      reject(Object.assign(new Error('Falha de rede ao consultar a Fábrica: ' + error.message), { statusCode: 502 }));
    });
    if (options.body && options.body.length) request.write(options.body);
    request.end();
  });
}

function upstreamJsonError(req, res, status, message, extra) {
  const body = {
    status: 'error',
    code: String(status),
    messages: [message],
    result: null
  };
  if (extra) Object.assign(body, extra);
  sendJson(req, res, status, body);
}

function forwardResponse(req, res, result) {
  cors(req, res);
  securityHeaders(res);
  res.statusCode = result.status;
  const contentType = result.headers['content-type'];
  if (contentType) res.setHeader('Content-Type', contentType);
  const contentLength = result.body.length;
  res.setHeader('Content-Length', contentLength);
  res.setHeader('Cache-Control', 'no-store');
  if (result.headers['retry-after']) res.setHeader('Retry-After', result.headers['retry-after']);
  if (result.headers['x-request-id']) res.setHeader('X-Request-Id', result.headers['x-request-id']);
  res.end(result.body);
}

function routeFor(method, pathname) {
  return ROUTES.find((route) => route.method === method && route.path === pathname);
}

async function handleApi(req, res, parsed) {
  const pathname = parsed.pathname.replace(/\/+$/, '') || '/';

  if (pathname === '/api/fabrica/health') {
    sendJson(req, res, 200, {
      ok: true,
      configured: Boolean(API_KEY),
      upstream: UPSTREAM,
      keyHeader: 'x-api-key',
      node: process.version
    });
    return;
  }

  if (!API_KEY) {
    upstreamJsonError(req, res, 503, 'FABRICA_API_KEY não configurada no servidor local. Crie .env a partir de .env.example e reinicie.');
    return;
  }

  if (req.method === 'GET' && pathname === '/api/fabrica/fundamentos') {
    const result = await requestUpstream({ method: 'GET', target: '/rgps/api/v1/fundamentos', search: parsed.search, timeout: 60000 });
    forwardResponse(req, res, result);
    return;
  }

  if (req.method === 'GET' && pathname === '/api/fabrica/marcos-temporais') {
    const result = await requestUpstream({ method: 'GET', target: '/rgps/api/v1/marcos-temporais', search: parsed.search, timeout: 60000 });
    forwardResponse(req, res, result);
    return;
  }

  if (req.method === 'GET' && pathname === '/api/fabrica/calculadoras') {
    const result = await requestUpstream({ method: 'GET', target: '/apps/api/v1/calculadoras', search: parsed.search, timeout: 60000 });
    forwardResponse(req, res, result);
    return;
  }

  const docMatch = pathname.match(/^\/api\/fabrica\/documentos\/([a-z]+)$/);
  if (req.method === 'POST' && docMatch) {
    const type = docMatch[1];
    if (!DOCUMENT_TYPES.has(type)) {
      upstreamJsonError(req, res, 404, 'Tipo de documento não suportado: ' + type + '.');
      return;
    }
    const body = await readBody(req, MAX_UPLOAD_BYTES);
    const result = await requestUpstream({
      method: 'POST',
      target: '/rgps/api/v1/documentos/' + type + '/ler',
      search: parsed.search,
      body,
      contentType: req.headers['content-type'] || 'application/octet-stream',
      timeout: 180000
    });
    forwardResponse(req, res, result);
    return;
  }

  if (req.method === 'GET' && pathname.startsWith('/api/fabrica/indices/')) {
    const name = pathname.slice('/api/fabrica/indices/'.length);
    const table = /^tabela\/\d{4}-\d{2}$/.test(name);
    if (!table && !INDEX_TYPES.has(name)) {
      upstreamJsonError(req, res, 404, 'Índice não suportado: ' + name + '.');
      return;
    }
    const result = await requestUpstream({
      method: 'GET',
      target: '/indices/api/v1/' + name,
      search: parsed.search,
      timeout: 60000
    });
    forwardResponse(req, res, result);
    return;
  }

  const jobMatch = pathname.match(/^\/api\/fabrica\/rmi\/jobs\/([^/]+)(\/resultado)?$/);
  if (req.method === 'GET' && jobMatch) {
    const suffix = jobMatch[2] || '';
    const result = await requestUpstream({
      method: 'GET',
      target: '/rgps/api/v1/calculos-rmi/jobs/' + encodeURIComponent(jobMatch[1]) + suffix,
      search: parsed.search,
      timeout: 60000
    });
    forwardResponse(req, res, result);
    return;
  }

  const tableMatch = pathname.match(/^\/api\/fabrica\/indices\/tabela\/([^/]+)$/);
  if (req.method === 'GET' && tableMatch) {
    const result = await requestUpstream({
      method: 'GET',
      target: '/indices/api/v1/tabela/' + encodeURIComponent(tableMatch[1]),
      search: parsed.search,
      timeout: 60000
    });
    forwardResponse(req, res, result);
    return;
  }

  if (req.method === 'GET' && (pathname === '/api/fabrica/documentos-legais' || pathname.startsWith('/api/fabrica/documentos-legais/'))) {
    const suffix = pathname.slice('/api/fabrica/documentos-legais'.length);
    const result = await requestUpstream({
      method: 'GET',
      target: '/apps/api/v1/documentos-legais' + suffix,
      search: parsed.search,
      timeout: 60000
    });
    forwardResponse(req, res, result);
    return;
  }

  if (req.method === 'POST' && pathname === '/api/fabrica/mcp') {
    const body = await readBody(req, MAX_JSON_BYTES);
    const result = await requestUpstream({
      method: 'POST',
      target: '/mcp',
      search: parsed.search,
      body,
      contentType: 'application/json',
      accept: req.headers.accept || 'application/json, text/event-stream',
      timeout: 120000
    });
    forwardResponse(req, res, result);
    return;
  }

  const route = routeFor(req.method, pathname);
  if (route) {
    const multipart = route.kind === 'multipart';
    const body = await readBody(req, multipart ? MAX_UPLOAD_BYTES : MAX_JSON_BYTES);
    const result = await requestUpstream({
      method: 'POST',
      target: route.target,
      search: parsed.search,
      body,
      contentType: multipart ? (req.headers['content-type'] || 'application/octet-stream') : 'application/json',
      timeout: route.timeout
    });
    forwardResponse(req, res, result);
    return;
  }

  upstreamJsonError(req, res, 404, 'Rota local não encontrada: ' + req.method + ' ' + pathname + '.');
}

function serveStatic(req, res, parsed) {
  if (req.method !== 'GET' && req.method !== 'HEAD') {
    res.statusCode = 405;
    res.setHeader('Allow', 'GET, HEAD');
    res.end('Método não permitido.');
    return;
  }
  let pathname;
  try {
    pathname = decodeURIComponent(parsed.pathname);
  } catch (error) {
    res.statusCode = 400;
    res.end('Caminho inválido.');
    return;
  }
  if (pathname === '/') pathname = '/index.html';
  const normalized = path.normalize(pathname).replace(/^(\.\.[\/\\])+/, '');
  const file = path.join(WEB, normalized);
  if (!file.startsWith(WEB + path.sep) && file !== WEB) {
    res.statusCode = 403;
    res.end('Acesso negado.');
    return;
  }
  fs.stat(file, (error, stat) => {
    if (error || !stat.isFile()) {
      res.statusCode = 404;
      res.setHeader('Content-Type', 'text/plain; charset=utf-8');
      res.end('Arquivo não encontrado.');
      return;
    }
    const ext = path.extname(file).toLowerCase();
    res.statusCode = 200;
    res.setHeader('Content-Type', MIME[ext] || 'application/octet-stream');
    res.setHeader('Content-Length', stat.size);
    res.setHeader('Cache-Control', 'no-store');
    res.setHeader('X-Content-Type-Options', 'nosniff');
    if (req.method === 'HEAD') {
      res.end();
      return;
    }
    fs.createReadStream(file).pipe(res);
  });
}

const server = http.createServer(async (req, res) => {
  const started = Date.now();
  let parsed;
  try {
    parsed = new URL(req.url || '/', 'http://' + (req.headers.host || 'localhost'));
  } catch (error) {
    res.statusCode = 400;
    res.end('Requisição inválida.');
    return;
  }

  if (req.method === 'OPTIONS') {
    cors(req, res);
    securityHeaders(res);
    res.statusCode = 204;
    res.end();
    return;
  }

  try {
    if (parsed.pathname.startsWith('/api/referencias/')) {
      await handleReferencias(req, res, parsed);
    } else if (parsed.pathname.startsWith('/api/fabrica/')) {
      await handleApi(req, res, parsed);
    } else {
      serveStatic(req, res, parsed);
    }
  } catch (error) {
    const status = Number(error.statusCode) || 502;
    if (!res.headersSent) upstreamJsonError(req, res, status, error.message || 'Falha no proxy local.');
    else res.end();
  } finally {
    const ms = Date.now() - started;
    console.log(req.method + ' ' + parsed.pathname + ' ' + res.statusCode + ' ' + ms + 'ms');
  }
});

server.on('clientError', (error, socket) => {
  if (socket.writable) socket.end('HTTP/1.1 400 Bad Request\r\n\r\n');
});

server.listen(PORT, HOST, () => {
  console.log('');
  console.log('previdenciar.io · proxy local da Fábrica de Cálculos');
  console.log('  Web:  http://' + HOST + ':' + PORT + '/');
  console.log('  API:  ' + UPSTREAM);
  console.log('  Chave: ' + (API_KEY ? 'configurada no servidor' : 'AUSENTE — crie .env com FABRICA_API_KEY=...'));
  console.log('  A chave nunca é enviada ao navegador.');
  console.log('');
});
