import { Channel, invoke } from '@tauri-apps/api/core';
import {normalizeCaseForEngine} from './model';
import type { Boot, Workspace, Cnis, Calculation, Json, BatchExport } from './model';
/** Presentation choices for the generated documents. They never change the calculation. */
export interface Presentation {includeAppendix?:boolean;includeQuality?:boolean;includePlanning?:boolean}
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
export type KnowledgeBlock='skills'|'legislation'|'natjus'|'jurisprudence';
export interface KnowledgeHit{ref:string;id:string;titulo:string;tipo?:string;status?:string;identificacao?:string;url?:string|null;trecho?:string|null;score?:number}
export interface KnowledgeOutcome{summary:string;sources:AiSource[];value:{resultados?:KnowledgeHit[];conteudo?:string;titulo?:string;id?:string;url?:string|null;fonte?:string;aviso?:string;[key:string]:unknown}}
export const api={
 fetchEconomicSnapshot:(through:string)=>invoke<import('./pcd').EconomicSnapshot>('fetch_economic_snapshot',{through}),
 fetchLocalEconomicSnapshot:(through:string)=>invoke<import('./pcd').EconomicSnapshot>('local_economic_snapshot',{through}),
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
};
export function errorMessage(e:unknown):string{if(typeof e==='string')return e;if(e instanceof Error)return e.message;return 'Não foi possível concluir a operação. Nenhuma alteração foi confirmada.'}
