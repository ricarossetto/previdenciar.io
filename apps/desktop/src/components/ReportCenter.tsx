import {useEffect, useRef, useState} from 'react';
import type {BatchExport, Calculation, Workspace} from '../model';
import {api, errorMessage} from '../api';
import type {Presentation} from '../api';
import {Badge, Button, Field, Icon, Notice} from './ui';
import '../styles/reports.css';

const FORMATS = [
  {id:'pdf_detailed',title:'Relatório completo',detail:'Resultado, períodos, remunerações e anexos de conferência.'},
  {id:'pdf_summary',title:'Resumo do cálculo',detail:'Síntese dos valores e dos parâmetros utilizados.'},
  {id:'pdf_periods',title:'Tempo de contribuição',detail:'Vínculos, períodos e apuração informada pelo motor.'},
  {id:'pdf_salaries',title:'Remunerações',detail:'Competências e lançamentos, sem índices presumidos.'},
  {id:'pdf_memory',title:'Memória da RMI',detail:'Salário de benefício, coeficiente, memória e correção disponível.'},
  {id:'pdf_transitions',title:'Regras de transição',detail:'Comparação dos requisitos e dos valores produzidos.'},
  {id:'pdf_indicators',title:'Pendências e indicadores',detail:'Achados que precisam de conferência documental.'},
  {id:'pdf_quality',title:'Qualidade de segurado',detail:'Período de graça, prorrogação e carência por vínculo, com cálculo próprio.'},
];

export function pdfBlob(base64:string):Blob {
  if(typeof base64!=='string'||base64.length>17*1024*1024)throw new Error('Prévia PDF inválida ou maior que o limite.');
  const decoded=atob(base64);
  if(!decoded.startsWith('%PDF-'))throw new Error('O gerador não retornou um PDF válido.');
  return new Blob([Uint8Array.from(decoded,c=>c.charCodeAt(0))],{type:'application/pdf'});
}

type Props={draftCase?:Calculation;workspace:Workspace;selected:number|null;onSelect:(id:number|null)=>void;onNotify:(message:string,error?:boolean)=>void;onChangeCase?:(c:Calculation)=>void};
export function ReportCenter({workspace,selected,onSelect,onNotify,draftCase,onChangeCase}:Props){
  const[format,setFormat]=useState('pdf_detailed');
  const[personal,setPersonal]=useState(false);
  const[busy,setBusy]=useState<'preview'|'export'|'batch'|null>(null);
  const[pdfUrl,setPdfUrl]=useState('');
  const[pdfBytes,setPdfBytes]=useState(0);
  const[error,setError]=useState('');
  const[batch,setBatch]=useState<string[]>([]);
  const[batchResult,setBatchResult]=useState<BatchExport|null>(null);
  const[title,setTitle]=useState('');
  const[presentation,setPresentation]=useState<Presentation>({includeAppendix:true,includeQuality:true,includePlanning:true});
  const generation=useRef(0),urlRef=useRef(''),lastCase=useRef<number|null>(null);
  const calculation=draftCase??workspace.calculations.find(c=>c.id===selected)??null;
  const choices=draftCase?[draftCase]:workspace.calculations;
  const description=FORMATS.find(f=>f.id===format);
  const isPdf=format.startsWith('pdf_');
  const presentationKey=String(presentation.includeAppendix)+'|'+String(presentation.includeQuality)+'|'+String(presentation.includePlanning);
  useEffect(()=>{
    const id=calculation?.id??null;
    if(id!==lastCase.current){lastCase.current=id;setTitle(calculation?.options.report_title??'');}
  },[calculation]);
  useEffect(()=>{
    generation.current+=1;
    if(urlRef.current){URL.revokeObjectURL(urlRef.current);urlRef.current='';}
    setPdfUrl('');setPdfBytes(0);setError('');setBusy(null);setBatchResult(null);
    return()=>{generation.current+=1;if(urlRef.current){URL.revokeObjectURL(urlRef.current);urlRef.current='';}};
  },[calculation,format,personal,presentationKey]);
  function toggleBatch(id:string){setBatch(current=>current.includes(id)?current.filter(value=>value!==id):[...current,id])}
  const allBatchSelected=batch.length===FORMATS.length;
  function applyTitle(){
    if(!calculation||!onChangeCase)return;
    onChangeCase({...calculation,options:{...calculation.options,report_title:title.trim()}});
    onNotify('Título do relatório salvo no cálculo. Ele passa a ser usado nas próximas prévias e exportações.');
  }
  async function preview(){
    if(!calculation||!isPdf||busy)return;
    const request=++generation.current;setBusy('preview');setError('');
    try{
      const encoded=await api.previewReport(calculation,workspace,format,personal,presentation);
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
      const destination=await api.exportReport(calculation,workspace,format,personal,presentation);
      if(request===generation.current&&destination)onNotify(draftCase?'PDF detalhado gerado localmente. Confira os avisos e o estado da hipótese.':'Relatório exportado no destino escolhido.');
    }catch(e){if(request===generation.current)setError(errorMessage(e));}
    finally{if(request===generation.current)setBusy(null);}
  }
  async function exportBatch(){
    if(!calculation||!batch.length||busy)return;
    const request=++generation.current;setBusy('batch');setError('');setBatchResult(null);
    try{
      const result=await api.exportReportsBatch(calculation,workspace,batch,personal,presentation);
      if(request!==generation.current||!result)return;
      setBatchResult(result);
      if(result.errors.length){
        setError(result.errors.map(item=>item.message).join(' '));
        onNotify(result.files.length?result.files.length+' PDF(s) exportados; '+result.errors.length+' falha(s). Revise a pasta e as mensagens.':'Nenhum PDF foi exportado. Revise as falhas apresentadas.',true);
      }else onNotify(result.files.length+' PDF(s) exportados em lote. Confira cada arquivo antes de compartilhar.');
    }catch(e){if(request===generation.current)setError(errorMessage(e));}
    finally{if(request===generation.current)setBusy(null);}
  }
  return <section className="report-center" aria-label="Central de relatórios">
    <aside className="report-controls">
      <div className="report-controls__heading"><Icon name="file"/><h2>Preparar relatório</h2></div>
      <label className="field"><span>Cálculo selecionado</span><select aria-label="Selecionar cálculo" value={draftCase?.id??selected??''} disabled={!!draftCase} onChange={e=>onSelect(e.target.value?Number(e.target.value):null)}><option value="">Escolha um cálculo</option>{choices.map(c=><option key={c.id} value={c.id}>{c.name||'Cálculo sem nome'}</option>)}</select></label>
      <label className="field"><span>{draftCase?'Tipo de relatório PDF':'Formato'}</span><select aria-label={draftCase?'Tipo de relatório PDF':'Formato'} value={format} onChange={event=>setFormat(event.target.value)}>{FORMATS.map(item=><option key={item.id} value={item.id}>{item.title}</option>)}{!draftCase&&<optgroup label="Exportar dados"><option value="html">Página HTML</option><option value="csv">Planilha CSV</option><option value="json">Dados JSON</option></optgroup>}</select></label>
      {description&&<p className="report-help">{description.detail}</p>}
      <label className="checkline report-privacy"><input type="checkbox" checked={personal} onChange={e=>setPersonal(e.target.checked)}/>Incluir identificação pessoal no relatório local</label>
      <p className="report-help">Sem essa opção, nome e identificadores ficam ocultos. Datas e valores continuam sensíveis.</p><details className="report-help"><summary>Outros modelos dos documentos originais</summary><p>Retorno do investimento não está disponível neste fluxo. A qualidade de segurado é apurada pelo próprio motor, por vínculo e com a regra legal declarada, e nunca copiada de outro segurado.</p></details>
    </aside>
    <div className="report-workspace">
      <header className="report-toolbar"><div><h2>{description?.title??'Exportação de dados'}</h2><p>{calculation?.name??'Selecione um cálculo para começar.'}</p></div><div className="report-toolbar__actions" data-ui-actions><Button disabled={!calculation||!isPdf||!!busy} onClick={()=>void preview()}>{busy==='preview'?'Gerando prévia…':'Pré-visualizar PDF'}</Button><Button primary disabled={!calculation||!!busy} onClick={()=>void exportFile()}>{busy==='export'?'Exportando…':draftCase?(format==='pdf_detailed'?'Gerar relatório PDF detalhado':'Gerar PDF'):'Exportar arquivo'}<Icon name="file" size={16}/></Button></div></header>
      {error&&<Notice tone="error">{error}</Notice>}
      <fieldset className="report-batch">
        <legend>Exportar vários relatórios</legend>
        <p className="report-help">Um arquivo PDF por tipo selecionado, na pasta que você escolher. Nomes fixos e arquivos existentes não são sobrescritos; o lote continua mesmo se um tipo falhar.</p>
        <div className="report-batch__list">{FORMATS.map(item=><label key={item.id} className="checkline"><input type="checkbox" checked={batch.includes(item.id)} onChange={()=>toggleBatch(item.id)} disabled={!!busy}/>{item.title}</label>)}</div>
        <div className="report-batch__actions" data-ui-actions><Button disabled={!!busy} onClick={()=>setBatch(allBatchSelected?[]:FORMATS.map(item=>item.id))}>{allBatchSelected?'Limpar seleção':'Selecionar todos'}</Button><Button primary disabled={!calculation||!batch.length||!!busy} onClick={()=>void exportBatch()}>{busy==='batch'?'Exportando lote…':'Exportar '+(batch.length?batch.length+' ':'')+'PDF(s)'}<Icon name="file" size={16}/></Button></div>
        {batchResult&&<div className="report-batch__result" role="status"><strong>Pasta escolhida:</strong> <span>{batchResult.directory}</span><ul>{batchResult.files.map(item=><li key={item.format}>{item.path.split(/[\\/]/).pop()}</li>)}{batchResult.errors.map(item=><li key={item.format} className="report-batch__error">{item.message}</li>)}</ul></div>}
      </fieldset>
      <fieldset className="report-presentation">
        <legend>Apresentação e título</legend>
        <Field label="Título do relatório" value={title} onChange={setTitle} placeholder="Ex.: RMI — revisão de origem (2026)"/>
        <div className="report-presentation__actions" data-ui-actions><Button disabled={!calculation||!onChangeCase||!!busy} onClick={applyTitle} title={!onChangeCase?'Este relatório não está vinculado a um cálculo editável.':undefined}>Aplicar título ao cálculo</Button></div>
        <p className="report-help">O título substitui o nome padrão do tipo nos documentos. Ele não altera o cálculo nem o resultado.</p>
        <div className="report-presentation__list">
          <label className="checkline"><input type="checkbox" checked={presentation.includeAppendix??true} onChange={e=>setPresentation(current=>({...current,includeAppendix:e.target.checked}))} disabled={!!busy}/>Incluir o anexo técnico de origem (relatório completo)</label>
          <label className="checkline"><input type="checkbox" checked={presentation.includeQuality??true} onChange={e=>setPresentation(current=>({...current,includeQuality:e.target.checked}))} disabled={!!busy}/>Incluir a conferência de qualidade por vínculo</label>
          <label className="checkline"><input type="checkbox" checked={presentation.includePlanning??true} onChange={e=>setPresentation(current=>({...current,includePlanning:e.target.checked}))} disabled={!!busy}/>Incluir as premissas de planejamento</label>
        </div>
        <p className="report-help">As opções de apresentação mudam apenas o que é impresso. Cada seção continua disponível nos outros tipos de relatório.</p>
      </fieldset>
      <div className="report-document-status"><Badge tone={pdfUrl?'good':'neutral'}>{pdfUrl?'PDF gerado':busy==='preview'?'Gerando documento':'Prévia não gerada'}</Badge><span>{pdfUrl?((pdfBytes/1024).toFixed(0)+' KB · documento pronto para conferir'):'A prévia usa os dados atuais do cálculo.'}</span></div>
      <div className="report-paper-workspace" aria-busy={busy==='preview'}>
        {pdfUrl?<><iframe className="report-pdf-frame" title="Prévia do relatório PDF" src={pdfUrl}/><p className="report-preview-fallback">Se o visualizador não abrir neste computador, <a href={pdfUrl} download="previus-previa.pdf">baixar esta prévia em PDF</a>.</p></>:<div className="report-empty-document"><Icon name="file" size={38}/><h3>{busy==='preview'?'Preparando o documento…':isPdf?'Confira o documento antes de exportar':'Formato de dados selecionado'}</h3><p>{busy==='preview'?'O motor está gerando as páginas da hipótese selecionada.':isPdf?'Escolha um cálculo e gere a prévia. Você verá o PDF de verdade, com páginas e tabelas, não uma lista de dados técnicos.':'HTML, CSV e JSON continuam disponíveis para exportação. A prévia paginada é exclusiva dos formatos PDF.'}</p></div>}
      </div>
      <p className="report-footer-note">Resultados experimentais e pendências permanecem identificados. Nenhuma ausência é convertida em benefício concedido ou valor zero.</p>
    </div>
  </section>;
}
