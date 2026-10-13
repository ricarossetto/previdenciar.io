import {useEffect, useRef, useState} from 'react';
import type {Calculation, Workspace} from '../model';
import {api, errorMessage} from '../api';
import {Badge, Button, Icon, Notice, Select} from './ui';
import '../styles/reports.css';

const FORMATS = [
  {id:'pdf_detailed',title:'Relatório completo',detail:'Resultado, períodos, remunerações e anexos de conferência.'},
  {id:'pdf_summary',title:'Resumo do cálculo',detail:'Síntese dos valores e dos parâmetros utilizados.'},
  {id:'pdf_periods',title:'Tempo de contribuição',detail:'Vínculos, períodos e apuração informada pelo motor.'},
  {id:'pdf_salaries',title:'Remunerações',detail:'Competências e lançamentos, sem índices presumidos.'},
  {id:'pdf_memory',title:'Memória da RMI',detail:'Salário de benefício, coeficiente, memória e correção disponível.'},
  {id:'pdf_transitions',title:'Regras de transição',detail:'Comparação dos requisitos e dos valores produzidos.'},
  {id:'pdf_indicators',title:'Pendências e indicadores',detail:'Achados que precisam de conferência documental.'},
];
const ALL_FORMATS = [...FORMATS.map(f=>f.id),'html','csv','json'];

export function pdfBlob(base64:string):Blob {
  if(typeof base64!=='string'||base64.length>17*1024*1024)throw new Error('Prévia PDF inválida ou maior que o limite.');
  const decoded=atob(base64);
  if(!decoded.startsWith('%PDF-'))throw new Error('O gerador não retornou um PDF válido.');
  return new Blob([Uint8Array.from(decoded,c=>c.charCodeAt(0))],{type:'application/pdf'});
}

type Props={draftCase?:Calculation;workspace:Workspace;selected:number|null;onSelect:(id:number|null)=>void;onNotify:(message:string,error?:boolean)=>void};
export function ReportCenter({workspace,selected,onSelect,onNotify,draftCase}:Props){
  const[format,setFormat]=useState('pdf_detailed');
  const[personal,setPersonal]=useState(false);
  const[busy,setBusy]=useState<'preview'|'export'|null>(null);
  const[pdfUrl,setPdfUrl]=useState('');
  const[pdfBytes,setPdfBytes]=useState(0);
  const[error,setError]=useState('');
  const generation=useRef(0),urlRef=useRef('');
  const calculation=draftCase??workspace.calculations.find(c=>c.id===selected)??null;
  const choices=draftCase?[draftCase]:workspace.calculations;
  const description=FORMATS.find(f=>f.id===format);
  const isPdf=format.startsWith('pdf_');
  useEffect(()=>{
    generation.current+=1;
    if(urlRef.current){URL.revokeObjectURL(urlRef.current);urlRef.current='';}
    setPdfUrl('');setPdfBytes(0);setError('');setBusy(null);
    return()=>{generation.current+=1;if(urlRef.current){URL.revokeObjectURL(urlRef.current);urlRef.current='';}};
  },[calculation,format,personal]);
  async function preview(){
    if(!calculation||!isPdf||busy)return;
    const request=++generation.current;setBusy('preview');setError('');
    try{
      const encoded=await api.previewReport(calculation,workspace,format,personal);
      if(request!==generation.current)return;
      const blob=pdfBlob(encoded),url=URL.createObjectURL(blob);
      if(urlRef.current)URL.revokeObjectURL(urlRef.current);
      urlRef.current=url;setPdfUrl(url);setPdfBytes(blob.size);
    }catch(e){if(request===generation.current)setError(errorMessage(e));}
    finally{if(request===generation.current)setBusy(null);}
  }
  async function exportFile(){
    if(!calculation||busy)return;
    const request=++generation.current;setBusy('export');setError('');
    try{
      const destination=await api.exportReport(calculation,workspace,format,personal);
      if(request===generation.current&&destination)onNotify(draftCase?'PDF detalhado gerado localmente. Confira os avisos e o estado da hipótese.':'Relatório exportado no destino escolhido.');
    }catch(e){if(request===generation.current)setError(errorMessage(e));}
    finally{if(request===generation.current)setBusy(null);}
  }
  return <section className="report-center" aria-label="Central de relatórios">
    <aside className="report-controls">
      <div className="report-controls__heading"><Icon name="file"/><h2>Preparar relatório</h2></div>
      <label className="field"><span>Cálculo selecionado</span><select aria-label="Selecionar cálculo" value={draftCase?.id??selected??''} disabled={!!draftCase} onChange={e=>onSelect(e.target.value?Number(e.target.value):null)}><option value="">Escolha um cálculo</option>{choices.map(c=><option key={c.id} value={c.id}>{c.name||'Cálculo sem nome'}</option>)}</select></label>
      <Select label={draftCase?"Tipo de relatório PDF":"Formato"} value={format} onChange={setFormat} items={draftCase?FORMATS.map(f=>f.id):ALL_FORMATS}/>
      <div className="report-template-list" role="group" aria-label="Modelos de relatório">{FORMATS.map(f=><button key={f.id} className={'report-template '+(format===f.id?'is-selected':'')} aria-pressed={format===f.id} onClick={()=>setFormat(f.id)}><span>{f.title}</span><small>{f.detail}</small></button>)}</div>
      <label className="checkline report-privacy"><input type="checkbox" checked={personal} onChange={e=>setPersonal(e.target.checked)}/>Incluir identificação pessoal no relatório local</label>
      <p className="report-help">Sem essa opção, nome e identificadores ficam ocultos. Datas e valores continuam sensíveis.</p><details className="report-help"><summary>Outros modelos dos documentos originais</summary><p>Retorno do investimento e qualidade de segurado não estão disponíveis neste fluxo. Seus valores e gráficos não são copiados de outro segurado.</p></details>
    </aside>
    <div className="report-workspace">
      <header className="report-toolbar"><div><h2>{description?.title??'Exportação de dados'}</h2><p>{calculation?.name??'Selecione um cálculo para começar.'}</p></div><div className="report-toolbar__actions" data-ui-actions><Button disabled={!calculation||!isPdf||!!busy} onClick={()=>void preview()}>{busy==='preview'?'Gerando prévia…':'Pré-visualizar PDF'}</Button><Button primary disabled={!calculation||!!busy} onClick={()=>void exportFile()}>{busy==='export'?'Exportando…':draftCase?(format==='pdf_detailed'?'Gerar relatório PDF detalhado':'Gerar PDF'):'Exportar arquivo'}<Icon name="file" size={16}/></Button></div></header>
      {error&&<Notice tone="error">{error}</Notice>}
      <div className="report-document-status"><Badge tone={pdfUrl?'good':'neutral'}>{pdfUrl?'PDF gerado':busy==='preview'?'Gerando documento':'Prévia não gerada'}</Badge><span>{pdfUrl?((pdfBytes/1024).toFixed(0)+' KB · mesmo gerador da exportação'):'Os valores são recalculados pelo Rust, não retirados de um JSON salvo.'}</span></div>
      <div className="report-paper-workspace" aria-busy={busy==='preview'}>
        {pdfUrl?<><iframe className="report-pdf-frame" title="Prévia do relatório PDF" src={pdfUrl}/><p className="report-preview-fallback">Se o visualizador não abrir neste computador, <a href={pdfUrl} download="previus-previa.pdf">baixar esta prévia em PDF</a>.</p></>:<div className="report-empty-document"><Icon name="file" size={38}/><h3>{busy==='preview'?'Preparando o documento…':isPdf?'Confira o documento antes de exportar':'Formato de dados selecionado'}</h3><p>{busy==='preview'?'O motor está gerando as páginas da hipótese selecionada.':isPdf?'Escolha um cálculo e gere a prévia. Você verá o PDF de verdade, com páginas e tabelas, não uma lista de dados técnicos.':'HTML, CSV e JSON continuam disponíveis para exportação. A prévia paginada é exclusiva dos formatos PDF.'}</p></div>}
      </div>
      <p className="report-footer-note">Resultados experimentais e pendências permanecem identificados. Nenhuma ausência é convertida em benefício concedido ou valor zero.</p>
    </div>
  </section>;
}
