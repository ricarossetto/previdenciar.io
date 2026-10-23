import { invoke } from '@tauri-apps/api/core';
import {normalizeCaseForEngine} from './model';
import type { Boot, Workspace, Cnis, Calculation, Json, BatchExport } from './model';
/** Presentation choices for the generated documents. They never change the calculation. */
export interface Presentation {includeAppendix?:boolean;includeQuality?:boolean;includePlanning?:boolean}
export const api={
 fetchEconomicSnapshot:(through:string)=>invoke<import('./pcd').EconomicSnapshot>('fetch_economic_snapshot',{through}),
 fetchLocalEconomicSnapshot:(through:string)=>invoke<import('./pcd').EconomicSnapshot>('local_economic_snapshot',{through}),
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
};
export function errorMessage(e:unknown):string{if(typeof e==='string')return e;if(e instanceof Error)return e.message;return 'Não foi possível concluir a operação. Nenhuma alteração foi confirmada.'}
