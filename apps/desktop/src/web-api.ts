import { normalizeCaseForEngine } from './model';
import { clearSession, refreshSession, storedToken } from './web-auth-store';
import type {
  AiChatRequest, AiDataStatus, AiEvent, AiImportSummary, AiKeyStatus, AiProviderPreset, AiTestResult,
  CasePdfText, KnowledgeBlock, KnowledgeOutcome, Presentation,
} from './api';
import type { BatchExport, Boot, Calculation, Cnis, Json, Workspace } from './model';
import type { EconomicSnapshot } from './pcd';

const API = '/api';

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

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const response = await authorizedFetch(API + path, {
    method,
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let parsed: unknown = null;
  try { parsed = text ? JSON.parse(text) : null; } catch { /* non-JSON response */ }
  if (!response.ok) {
    const message = parsed && typeof parsed === 'object' && 'error' in parsed
      ? String((parsed as { error: unknown }).error)
      : `A operaÃ§Ã£o falhou (${response.status}).`;
    throw new Error(message);
  }
  return parsed as T;
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
  throw new Error(`${feature} ainda nÃ£o estÃ¡ disponÃ­vel na versÃ£o web.`);
}

export const webApi = {
  fetchEconomicSnapshot: (through: string) => request<EconomicSnapshot>('GET', `/pcd/snapshot?through=${encodeURIComponent(through)}`),
  fetchLocalEconomicSnapshot: (through: string) => request<EconomicSnapshot>('GET', `/pcd/snapshot?through=${encodeURIComponent(through)}`),
  conferirFabricaRmi: () => unsupported('ConferÃªncia na FÃ¡brica'),
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
    const parsed = text ? JSON.parse(text) : null;
    if (!response.ok) throw new Error(parsed?.error ?? `A importaÃ§Ã£o falhou (${response.status}).`);
    return parsed as Cnis;
  },
  importPath: (_path: string) => unsupported('ImportaÃ§Ã£o por caminho local'),
  calculate: (calculation: Calculation) => request<Json>('POST', '/calculate', { calculation: normalizeCaseForEngine(calculation) }),
  rawRgps: (_input: Json) => unsupported('Motor RGPS direto'),
  tool: (tool: string, input: Json) => request<Json>('POST', `/tools/${encodeURIComponent(tool)}`, input),
  finance: (tool: string, input: Json) => request<Json>('POST', `/finance/${encodeURIComponent(tool)}`, input),
  previewReport: async (calculation: Calculation, workspace: Workspace, format: string, includePersonal: boolean, presentation?: Presentation) => {
    const response = await authorizedFetch(`${API}/reports/preview`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ calculation: normalizeCaseForEngine(calculation), workspace, format, include_personal: includePersonal, presentation: presentation ?? null }),
    });
    if (!response.ok) {
      const text = await response.text();
      const parsed = text ? JSON.parse(text) : null;
      throw new Error(parsed?.error ?? `A prÇ¸via falhou (${response.status}).`);
    }
    return URL.createObjectURL(await response.blob());
  },
  saveReport: async (calculation: Calculation, workspace: Workspace, format: string, includePersonal: boolean, presentation?: Presentation) => request<{ id: string; format: string; sizeBytes: number; sha256: string; expiresInDays: number }>('POST', '/reports/save', { calculation: normalizeCaseForEngine(calculation), workspace, format, include_personal: includePersonal, presentation: presentation ?? null }),

  exportReport: async (calculation: Calculation, workspace: Workspace, format: string, includePersonal: boolean, presentation?: Presentation) => {
    const response = await authorizedFetch(`${API}/reports/preview`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ calculation: normalizeCaseForEngine(calculation), workspace, format, include_personal: includePersonal, presentation: presentation ?? null }),
    });
    if (!response.ok) {
      const text = await response.text();
      const parsed = text ? JSON.parse(text) : null;
      throw new Error(parsed?.error ?? `A exportaï¿½ï¿½Çœo falhou (${response.status}).`);
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
      note: 'O envio usa a chave da plataforma e o consentimento explÃ­cito desta conversa.',
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
  aiDataStatus: () => Promise.resolve({ skills: 0, legislation: 0, natjus: 0, natjusText: 0, jurisprudence: 0, ready: false, message: 'Base de conhecimento ainda nÃ£o conectada ao servidor.' } as AiDataStatus),
  aiKnowledgeSearch: (_block: KnowledgeBlock, _query: string, _limit?: number) => unsupported('Busca na base'),
  aiKnowledgeRead: (_block: KnowledgeBlock, _id: string) => unsupported('Leitura da base'),
  pickCasePdfText: async () => {
    const file = await chooseFile('application/pdf,.pdf');
    if (!file) return null;
    const form = new FormData();
    form.append('file', file, file.name);
    const response = await authorizedFetch(`${API}/case-pdf/text`, { method: 'POST', body: form });
    const text = await response.text();
    const parsed = text ? JSON.parse(text) : null;
    if (!response.ok) throw new Error(parsed?.error ?? `A leitura do PDF falhou (${response.status}).`);
    return parsed as CasePdfText;
  },
  saveExportFile: (data: Uint8Array, extension: string, suggestedName: string) => Promise.resolve(download(data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer, 'application/octet-stream', `${suggestedName}.${extension}`)),
  aiImportData: (_includeNatjusPdf: boolean) => unsupported('ImportaÃ§Ã£o da base'),
  aiChat: async (requestBody: AiChatRequest, onEvent: (event: AiEvent) => void) => {
    const response = await authorizedFetch(`${API}/ai/chat`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(requestBody),
    });
    if (!response.ok) {
      const text = await response.text();
      let message = `O Copiloto falhou (${response.status}).`;
      try { message = JSON.parse(text)?.error ?? message; } catch { /* resposta nÃ£o JSON */ }
      throw new Error(message);
    }
    if (!response.body) throw new Error('O Copiloto nÃ£o retornou uma resposta legÃ­vel.');
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

