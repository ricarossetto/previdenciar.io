import { invoke } from '@tauri-apps/api/core';
import {normalizeCaseForEngine} from './model';
import type { Boot, Workspace, Cnis, Calculation, Json } from './model';
export const api={
 boot:()=>invoke<Boot>('bootstrap'),
 save:(workspace:Workspace)=>invoke<Workspace>('save_workspace',{workspace}),
 pickCnis:()=>invoke<Cnis|null>('pick_import_cnis'),
 importPath:(path:string)=>invoke<Cnis>('import_cnis_path',{path}),
 calculate:(calculation:Calculation)=>invoke<Json>('calculate_case',{calculation:normalizeCaseForEngine(calculation)}),
 rawRgps:(input:Json)=>invoke<Json>('calculate_rgps',{input}),
 tool:(tool:string,input:Json)=>invoke<Json>('run_tool',{tool,input}),
 finance:(tool:string,input:Json)=>invoke<Json>('run_finance',{tool,input}),
 previewReport:(calculation:Calculation,workspace:Workspace,format:string,includePersonal:boolean)=>invoke<string>('preview_report',{calculation:normalizeCaseForEngine(calculation),workspace,format,includePersonal}),
 exportReport:(calculation:Calculation,workspace:Workspace,format:string,includePersonal:boolean)=>invoke<string|null>('export_report',{calculation:normalizeCaseForEngine(calculation),workspace,format,includePersonal}),
 exportDocument:(text:string)=>invoke<string|null>('export_document',{text}),
 backup:()=>invoke<string|null>('backup_export'),
};
export function errorMessage(e:unknown):string{if(typeof e==='string')return e;if(e instanceof Error)return e.message;return 'Não foi possível concluir a operação. Nenhuma alteração foi confirmada.'}
