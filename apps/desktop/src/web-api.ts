import { normalizeCaseForEngine } from './model';
import { describeFailure, parseJsonSafe } from './http-errors';
import { clearSession, refreshSession, storedToken } from './web-auth-store';
import type {
  AiChatRequest, AiDataStatus, AiEvent, AiImportSummary, AiKeyStatus, AiProviderPreset, AiTestResult,
  CasePdfText, KnowledgeBlock, KnowledgeOutcome, Presentation, SavedReport, SavedReportStored, SavedReportUrl,
} from './api';
import type { BatchExport, Boot, Calculation, Cnis, Json, Workspace } from './model';
import type { EconomicSnapshot } from './pcd';

const API = '/api';

const MSG_OPERACAO = 'A operação falhou. Tente de novo em instantes.';

// Corpo de sucesso tem de ser um registro JSON. Texto vazio, página HTML,
// o literal null e lista não são dados: o servidor publica o SPA em rota
// desconhecida, então um 200 com HTML passaria por resposta boa.
// Nenhum endpoint usado aqui devolve lista no topo; quando isso existir, a
// validação precisa ser declarada no contrato do endpoint, não afrouxada aqui.
function readSuccessObject(texto: string, padrao: string): Record<string, unknown> {
  const limpo = texto.trim();
  if (!limpo || limpo.startsWith('<')) throw new Error(padrao);
  let valor: unknown;
  try {
    valor = JSON.parse(limpo);
  } catch {
    throw new Error(padrao);
  }
  if (!valor || typeof valor !== 'object' || Array.isArray(valor)) throw new Error(padrao);
  return valor as Record<string, unknown>;
}

// Endpoint cujo contrato admite lista no topo. Ainda exige JSON de objeto ou
// lista: vazio, HTML e null continuam recusados.
function readSuccessValue(texto: string, padrao: string): Record<string, unknown> | unknown[] {
  const limpo = texto.trim();
  if (!limpo || limpo.startsWith('<')) throw new Error(padrao);
  let valor: unknown;
  try {
    valor = JSON.parse(limpo);
  } catch {
    throw new Error(padrao);
  }
  if (!valor || typeof valor !== 'object') throw new Error(padrao);
  return valor as Record<string, unknown> | unknown[];
}

function authHeaders(extra?: HeadersInit): HeadersInit {
  const headers = new Headers(extra);
  const token = storedToken();
  if (token) headers.set('Authorization', `Bearer ${token}`);
  return headers;
}

async function authorizedFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const send = () => fetch(path, { ...init, credentials: 'include', headers: authHeaders(init.headers) });
  let response = await send();
  if (response.status === 401 && await refreshSession()) {
    response = await send();
  }
  if (response.status === 401) clearSession();
  return response;
}

type RequestOptions = {
  // Endpoint cujo contrato é uma lista no topo. Fora disso, lista é inválida.
  permitirLista?: boolean;
};

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  opcoes: RequestOptions = {},
): Promise<T> {
  const response = await authorizedFetch(API + path, {
    method,
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(describeFailure(response.status, parseJsonSafe(text), MSG_OPERACAO));
  }
  if (opcoes.permitirLista) return readSuccessValue(text, MSG_OPERACAO) as T;
  return readSuccessObject(text, MSG_OPERACAO) as T;
}

function chooseFile(accept: string): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = accept;
    input.onchange = () => resolve(input.files?.[0] ?? null);
    input.click();
  });
}

function download(bytes: BlobPart, type: string, name: string): string {
  const url = URL.createObjectURL(new Blob([bytes], { type }));
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = name;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return name;
}

async function unsupported(feature: string): Promise<never> {
  throw new Error(`${feature} ainda não está disponível na versão web.`);
}

export const webApi = {
  fetchEconomicSnapshot: (through: string) => request<EconomicSnapshot>('GET', `/pcd/snapshot?through=${encodeURIComponent(through)}`),
  fetchLocalEconomicSnapshot: (through: string) => request<EconomicSnapshot>('GET', `/pcd/snapshot?through=${encodeURIComponent(through)}`),
  conferirFabricaRmi: () => unsupported('Conferência na Fábrica'),
  prepareFromCnis: (calculation: Calculation, reference: string) =>
    request<{ calculation: Calculation; notes: string[] }>('POST', '/prepare-from-cnis', { calculation: normalizeCaseForEngine(calculation), reference }),
  boot: () => request<Boot>('GET', '/bootstrap'),
  save: (workspace: Workspace) => request<Workspace>('PUT', '/workspace', { workspace }),
  pickCnis: async () => {
    const file = await chooseFile('application/pdf,.pdf');
    if (!file) return null;
    const form = new FormData();
    form.append('file', file, file.name);
    const response = await authorizedFetch(`${API}/cnis/import`, { method: 'POST', body: form });
    const text = await response.text();
    const falha = 'A importação do CNIS falhou. Tente de novo em instantes.';
    if (!response.ok) throw new Error(describeFailure(response.status, parseJsonSafe(text), falha));
    // O importador devolve sempre um registro do CNIS. Lista, null ou HTML
    // aqui é resposta inválida, não um CNIS vazio.
    return readSuccessObject(text, falha) as unknown as Cnis;
  },
  importPath: (_path: string) => unsupported('Importação por caminho local'),
  calculate: (calculation: Calculation) => request<Json>('POST', '/calculate', { calculation: normalizeCaseForEngine(calculation) }),
  rawRgps: (_input: Json) => unsupported('Motor RGPS direto'),
  tool: (tool: string, input: Json) => request<Json>('POST', `/tools/${encodeURIComponent(tool)}`, input, { permitirLista: true }),
  finance: (tool: string, input: Json) => request<Json>('POST', `/finance/${encodeURIComponent(tool)}`, input, { permitirLista: true }),
  previewReport: async (calculation: Calculation, workspace: Workspace, format: string, includePersonal: boolean, presentation?: Presentation) => {
    const response = await authorizedFetch(`${API}/reports/preview`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ calculation: normalizeCaseForEngine(calculation), workspace, format, include_personal: includePersonal, presentation: presentation ?? null }),
    });
    if (!response.ok) {
      const text = await response.text();
      throw new Error(describeFailure(response.status, parseJsonSafe(text), 'A prévia do relatório falhou.'));
    }
    return URL.createObjectURL(await response.blob());
  },
  savedReports: {
    save: async (calculation: Calculation, workspace: Workspace, format: string, includePersonal: boolean, presentation?: Presentation) =>
      request<SavedReportStored>('POST', '/reports/save', { calculation: normalizeCaseForEngine(calculation), workspace, format, include_personal: includePersonal, presentation: presentation ?? null }),
    list: async () => {
      const data = await request<{ reports?: SavedReport[] }>('GET', '/reports/saved');
      return data.reports ?? [];
    },
    link: (id: string) => request<SavedReportUrl>('GET', `/reports/saved/${encodeURIComponent(id)}`),
    remove: async (id: string) => { await request<{ deleted?: boolean }>('DELETE', `/reports/saved/${encodeURIComponent(id)}`); },
  },

  exportReport: async (calculation: Calculation, workspace: Workspace, format: string, includePersonal: boolean, presentation?: Presentation) => {
    const response = await authorizedFetch(`${API}/reports/preview`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ calculation: normalizeCaseForEngine(calculation), workspace, format, include_personal: includePersonal, presentation: presentation ?? null }),
    });
    if (!response.ok) {
      const text = await response.text();
      throw new Error(describeFailure(response.status, parseJsonSafe(text), 'A exportação do relatório falhou.'));
    }
    return download(await response.blob(), 'application/pdf', `previdenciar-${format.replace(/^pdf_/, '')}.pdf`);
  },
  exportReportsBatch: async (calculation: Calculation, workspace: Workspace, formats: string[], includePersonal: boolean, presentation?: Presentation): Promise<BatchExport> => {
    const files: { format: string; path: string }[] = [];
    const errors: { format: string; message: string }[] = [];
    for (const format of formats) {
      try {
        const path = await webApi.exportReport(calculation, workspace, format, includePersonal, presentation);
        files.push({ format, path: path ?? '' });
      } catch (error) {
        errors.push({ format, message: error instanceof Error ? error.message : String(error) });
      }
    }
    return { directory: 'Downloads', files, errors };
  },
  exportDocument: (text: string) => Promise.resolve(download(text, 'text/plain;charset=utf-8', 'documento.txt')),
  backup: async () => {
    const boot = await request<Boot>('GET', '/bootstrap');
    return download(JSON.stringify(boot.workspace, null, 2), 'application/json', 'previdenciar-workspace.json');
  },
  aiCatalog: async () => {
    const status = await request<{ enabled: boolean; providerId: string; baseUrl: string; model: string; message: string | null }>('GET', '/ai/status');
    if (!status.enabled) return [];
    return [{
      id: status.providerId,
      name: 'Copiloto da plataforma',
      kind: 'openai',
      baseUrl: status.baseUrl,
      defaultModel: status.model,
      models: [status.model],
      keyRequired: true,
      note: 'O envio usa a chave da plataforma e o consentimento explícito desta conversa.',
    } as AiProviderPreset];
  },
  aiKeyStatus: async () => {
    const status = await request<{ enabled: boolean; providerId: string }>('GET', '/ai/status');
    return [{ providerId: status.providerId, hasKey: status.enabled }] as AiKeyStatus[];
  },
  aiSaveKey: () => unsupported('Chave de IA'),
  aiDeleteKey: () => unsupported('Chave de IA'),
  aiTest: () => unsupported('Teste de IA'),
  aiModels: async () => {
    const status = await request<{ model: string }>('GET', '/ai/status');
    return [status.model];
  },
  aiDataStatus: () => Promise.resolve({ skills: 0, legislation: 0, natjus: 0, natjusText: 0, jurisprudence: 0, ready: false, message: 'Base de conhecimento ainda não conectada ao servidor.' } as AiDataStatus),
  aiKnowledgeSearch: (_block: KnowledgeBlock, _query: string, _limit?: number) => unsupported('Busca na base'),
  aiKnowledgeRead: (_block: KnowledgeBlock, _id: string) => unsupported('Leitura da base'),
  pickCasePdfText: async () => {
    const file = await chooseFile('application/pdf,.pdf');
    if (!file) return null;
    const form = new FormData();
    form.append('file', file, file.name);
    const response = await authorizedFetch(`${API}/case-pdf/text`, { method: 'POST', body: form });
    const text = await response.text();
    const falha = 'A leitura do PDF falhou. Tente de novo em instantes.';
    if (!response.ok) throw new Error(describeFailure(response.status, parseJsonSafe(text), falha));
    return readSuccessObject(text, falha) as unknown as CasePdfText;
  },
  saveExportFile: (data: Uint8Array, extension: string, suggestedName: string) => Promise.resolve(download(data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer, 'application/octet-stream', `${suggestedName}.${extension}`)),
  aiImportData: (_includeNatjusPdf: boolean) => unsupported('Importação da base'),
  aiChat: async (requestBody: AiChatRequest, onEvent: (event: AiEvent) => void) => {
    const response = await authorizedFetch(`${API}/ai/chat`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(requestBody),
    });
    if (!response.ok) {
      const text = await response.text();
      throw new Error(describeFailure(response.status, parseJsonSafe(text), 'O Copiloto não respondeu. Tente de novo em instantes.'));
    }
    if (!response.body) throw new Error('O Copiloto não retornou uma resposta legível.');
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';
    for (;;) {
      const { done, value } = await reader.read();
      buffer += decoder.decode(value ?? new Uint8Array(), { stream: !done });
      let boundary = buffer.indexOf('\n\n');
      while (boundary >= 0) {
        const frame = buffer.slice(0, boundary);
        buffer = buffer.slice(boundary + 2);
        const data = frame.split('\n').filter((line) => line.startsWith('data:'))
          .map((line) => line.slice(5).trim()).join('\n');
        if (data) {
          const event = JSON.parse(data) as AiEvent | { type: 'error'; message: string };
          if (event.type === 'error') throw new Error(event.message);
          onEvent(event as AiEvent);
        }
        boundary = buffer.indexOf('\n\n');
      }
      if (done) break;
    }
  },
  aiCancel: (_requestId: string) => Promise.resolve(),
};

export type WebApi = typeof webApi;
export type WebCnis = Cnis;
export type WebKnowledgeOutcome = KnowledgeOutcome;
export type WebAiTestResult = AiTestResult;
export type WebBatchExport = BatchExport;
export type WebCasePdfText = CasePdfText;
export type WebAiImportSummary = AiImportSummary;

