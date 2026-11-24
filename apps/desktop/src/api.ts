import { Channel, invoke } from '@tauri-apps/api/core';
import {normalizeCaseForEngine} from './model';
import type { Boot, Workspace, Cnis, Calculation, Json, BatchExport } from './model';
import type { FabricaInput, FabricaSummary } from './fabrica';
/** Presentation choices for the generated documents. They never change the calculation. */
export interface Presentation {includeAppendix?:boolean;includeQuality?:boolean;includePlanning?:boolean}
export interface SavedReport {id:string;calculationId:number|null;format:string;sizeBytes:number;sha256:string;createdAt:string;expiresAt:string}
export interface SavedReportUrl {url:string;expiresInSeconds:number}
export interface SavedReportStored {id:string;format:string;sizeBytes:number;sha256:string;expiresInDays:number}
/** Saved PDFs live on the server account; the desktop build has no account, so it exposes null. */
export interface SavedReportsApi {
  save:(calculation:Calculation,workspace:Workspace,format:string,includePersonal:boolean,presentation?:Presentation)=>Promise<SavedReportStored>;
  list:()=>Promise<SavedReport[]>;
  link:(id:string)=>Promise<SavedReportUrl>;
  remove:(id:string)=>Promise<void>;
}
/** Contato do Atrium, já filtrado pelo que o usuário vê lá. */
export interface AtriumContact {id:string;name:string;cpf:string;email:string;phone:string;birth_date:string;sex:string;notes:string}
/** Processo do Atrium visível ao usuário. */
export interface AtriumProcess {id:string;number:string;title:string;client:string;contact_id:string;nb:string;phase:string;court:string;notes:string}
export interface AtriumWrite {id:string;created:boolean}
/** Troca de clientes e casos com o Atrium. Só existe no site; o desktop não tem conta. */
export interface AtriumApi {
  contacts:(query?:string)=>Promise<AtriumContact[]>;
  processes:(contactIds:string[])=>Promise<AtriumProcess[]>;
  sendContact:(contact:{name:string;cpf:string;email:string;phone:string;birth_date:string;sex:string;notes:string})=>Promise<AtriumWrite>;
  sendProcess:(process:{calculation_id:number;contact_id:string;atrium_id:string;title:string;nb:string;notes:string})=>Promise<AtriumWrite>;
}
export type AiProviderKind='openai'|'anthropic';
export interface AiProviderPreset{id:string;name:string;kind:AiProviderKind;baseUrl:string;defaultModel:string;models:string[];keyRequired:boolean;note:string}
export interface AiKeyStatus{providerId:string;hasKey:boolean}
export interface AiDataStatus{skills:number;legislation:number;natjus:number;natjusText:number;jurisprudence:number;ready:boolean;message:string}
export interface AiSource{label:string;title:string;url:string|null;kind:string}
export interface AiUsage{inputTokens:number|null;outputTokens:number|null}
export type AiEvent={type:'delta';text:string}|{type:'tool';name:string;status:'running'|'done'|'error';summary:string;sources:AiSource[]}|{type:'done';finish:string;usage:AiUsage|null};
export interface AiChatMessage{role:'user'|'assistant';content:string}
export type AiTask='chat'|'plan'|'draft'|'review'|'adversarial';
export interface AiChatRequest{requestId:string;providerId:string;kind:AiProviderKind;baseUrl:string;model:string;messages:AiChatMessage[];consent:boolean;toolsEnabled:boolean;maxTokens?:number|null;caseContext?:string|null;task?:AiTask}
export interface AiTestResult{ok:boolean;model:string;latencyMs:number;sample:string}
export interface CasePdfText{fileName:string;text:string;usedOcr:boolean;pages:number;hashSha256:string}
export interface AiImportSummary{skills:number;legislation:number;natjus:number;jurisprudence:number;natjusPdfIncluded:boolean;natjusText:number;natjusTextFailures:number;destination:string}
export interface FabricaStatus { configured: boolean; serverKeyAvailable: boolean; vaultConfigured?: boolean; sessionKeyConfigured?: boolean; upstream: string }
export interface FabricaRmiOptions { demonstrativo?: boolean; permalink?: boolean; fundamento?: boolean }
export interface FabricaTempoOptions { porGrupos?: boolean; demonstrativo?: boolean; permalink?: boolean }
export interface FabricaAtrasadosOptions { demonstrativo?: boolean; permalink?: boolean }
export type KnowledgeBlock='skills'|'legislation'|'natjus'|'jurisprudence';
export interface KnowledgeHit{ref:string;id:string;titulo:string;tipo?:string;status?:string;identificacao?:string;url?:string|null;trecho?:string|null;score?:number}
export interface KnowledgeOutcome{summary:string;sources:AiSource[];value:{resultados?:KnowledgeHit[];conteudo?:string;titulo?:string;id?:string;url?:string|null;fonte?:string;aviso?:string;[key:string]:unknown}}
const tauriApi={
 fetchEconomicSnapshot:(through:string)=>invoke<import('./pcd').EconomicSnapshot>('fetch_economic_snapshot',{through}),
 fetchLocalEconomicSnapshot:(through:string)=>invoke<import('./pcd').EconomicSnapshot>('local_economic_snapshot',{through}),
 conferirFabricaRmi:(input:FabricaInput,apiKey:string,options?:FabricaRmiOptions)=>options?invoke<FabricaSummary>('conferir_fabrica_rmi',{input,apiKey,...options}):invoke<FabricaSummary>('conferir_fabrica_rmi',{input,apiKey}),
 fabricaStatus:()=>invoke<FabricaStatus>('fabrica_status'),
 fabricaSaveKey:(apiKey:string)=>invoke<void>('fabrica_save_key',{apiKey}),
 fabricaClearKey:()=>invoke<void>('fabrica_clear_key'),
 calcularFabricaTempo:(input:FabricaInput|Json,apiKey?:string,options?:FabricaTempoOptions)=>invoke<Json>('calcular_fabrica_tempo',{input,apiKey:apiKey??null,porGrupos:options?.porGrupos??false,demonstrativo:options?.demonstrativo??null,permalink:options?.permalink??null}),
 calcularFabricaAtrasados:(body:Json,apiKey?:string,options?:FabricaAtrasadosOptions)=>invoke<Json>('calcular_fabrica_atrasados',{body,apiKey:apiKey??null,demonstrativo:options?.demonstrativo??false,permalink:options?.permalink??false}),
 lerDocumentoFabrica:async(tipo:'cnis'|'carta'|'rdctc'|'pap'|'prevjud',apiKey?:string,file?:File)=>file?invoke<Json>('ler_documento_fabrica',{tipo,fileName:file.name,data:Array.from(new Uint8Array(await file.arrayBuffer())),apiKey:apiKey??null}):invoke<Json>('ler_documento_fabrica',{tipo,apiKey:apiKey??null}),
 consultarFabricaIndices:(endpoint:'indice'|'fundamentos'|'marcos-temporais',nome?:string,query?:Record<string,string>,apiKey?:string)=>invoke<Json>('consultar_fabrica_indices',{endpoint,nome:nome??null,query:query??null,apiKey:apiKey??null}),
 prepareFromCnis:(calculation:Calculation,reference:string)=>invoke<{calculation:Calculation;notes:string[]}>('prepare_case_from_cnis',{calculation:normalizeCaseForEngine(calculation),reference}),
 boot:()=>invoke<Boot>('bootstrap'),
 save:(workspace:Workspace)=>invoke<Workspace>('save_workspace',{workspace}),
 pickCnis:()=>invoke<Cnis|null>('pick_import_cnis'),
 importPath:(path:string)=>invoke<Cnis>('import_cnis_path',{path}),
 calculate:(calculation:Calculation)=>invoke<Json>('calculate_case',{calculation:normalizeCaseForEngine(calculation)}),
 rawRgps:(input:Json)=>invoke<Json>('calculate_rgps',{input}),
 tool:(tool:string,input:Json)=>invoke<Json>('run_tool',{tool,input}),
 finance:(tool:string,input:Json)=>invoke<Json>('run_finance',{tool,input}),
 previewReport:(calculation:Calculation,workspace:Workspace,format:string,includePersonal:boolean,presentation?:Presentation)=>invoke<string>('preview_report',{calculation:normalizeCaseForEngine(calculation),workspace,format,includePersonal,presentation:presentation??null}),
 exportReport:(calculation:Calculation,workspace:Workspace,format:string,includePersonal:boolean,presentation?:Presentation)=>invoke<string|null>('export_report',{calculation:normalizeCaseForEngine(calculation),workspace,format,includePersonal,presentation:presentation??null}),
 exportReportsBatch:(calculation:Calculation,workspace:Workspace,formats:string[],includePersonal:boolean,presentation?:Presentation)=>invoke<BatchExport|null>('export_reports_batch',{calculation:normalizeCaseForEngine(calculation),workspace,formats,includePersonal,presentation:presentation??null}),
 exportDocument:(text:string)=>invoke<string|null>('export_document',{text}),
 backup:()=>invoke<string|null>('backup_export'),
 aiCatalog:()=>invoke<AiProviderPreset[]>('ai_provider_catalog'),
 aiKeyStatus:()=>invoke<AiKeyStatus[]>('ai_key_status'),
 aiSaveKey:(providerId:string,apiKey:string)=>invoke<void>('ai_save_key',{providerId,apiKey}),
 aiDeleteKey:(providerId:string)=>invoke<void>('ai_delete_key',{providerId}),
 aiTest:(providerId:string,kind:AiProviderKind,baseUrl:string,model:string)=>invoke<AiTestResult>('ai_test',{providerId,kind,baseUrl,model}),
 aiModels:(providerId:string,kind:AiProviderKind,baseUrl:string)=>invoke<string[]>('ai_models',{providerId,kind,baseUrl}),
 aiDataStatus:()=>invoke<AiDataStatus>('ai_data_status'),
 aiKnowledgeSearch:(block:KnowledgeBlock,query:string,limit=8)=>invoke<KnowledgeOutcome>('ai_knowledge_search',{block,query,limit}),
 aiKnowledgeRead:(block:KnowledgeBlock,id:string)=>invoke<KnowledgeOutcome>('ai_knowledge_read',{block,id}),
 pickCasePdfText:()=>invoke<CasePdfText|null>('pick_case_pdf_text'),
 saveExportFile:(data:Uint8Array,extension:string,suggestedName:string)=>invoke<string|null>('save_export_file',{data:Array.from(data),extension,suggestedName}),
 aiImportData:(includeNatjusPdf:boolean)=>invoke<AiImportSummary|null>('ai_import_data',{includeNatjusPdf}),
 aiChat:(request:AiChatRequest,onEvent:(event:AiEvent)=>void)=>{const channel=new Channel<AiEvent>();channel.onmessage=onEvent;return invoke<void>('ai_chat',{request,onEvent:channel})},
 aiCancel:(requestId:string)=>invoke<void>('ai_cancel',{requestId}),
 savedReports:null,
 atrium:null as AtriumApi|null,
};
export function errorMessage(e:unknown):string{if(typeof e==='string')return e;if(e instanceof Error)return e.message;return 'Não foi possível concluir a operação. Nenhuma alteração foi confirmada.'}
import { webApi } from './web-api';
export const api = import.meta.env.MODE === 'web' ? webApi : tauriApi;
