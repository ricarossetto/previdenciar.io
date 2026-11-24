import {useEffect,useId,useLayoutEffect,useRef,useState} from 'react';
import { EmptyFolder } from './lucian/LucianEmpty';
import {createPortal} from 'react-dom';
import {motion} from 'motion/react';
import {springs,useMotionPolicy} from './amicro/motion';
import type{ReactNode,ButtonHTMLAttributes}from'react';
// Ícones de traço único. Início, agenda e configurações vêm do Lucian UX/UI Prompt #417
// (MIT, https://lucianlabs.dev/uxui-prompt/#417), escalados de 18 para 24; os demais seguem o mesmo traço.
const L417=(d:ReactNode)=><g transform="scale(1.3333)" strokeWidth="1.2">{d}</g>;
const ICONS:Record<string,ReactNode>={
motion:<><path d="M4 7h6M4 12h4M4 17h6"/><path d="M14 5.5 20 12l-6 6.5"/></>,
menu:<path d="M4 7h16M4 12h16M4 17h10"/>,
home:L417(<path d="M3 8.2 9 3.2l6 5V15h-3.8v-3.8H6.8V15H3z"/>),
users:<><circle cx="9" cy="8" r="3.2"/><path d="M3.5 19.5a5.5 5.5 0 0 1 11 0"/><path d="M16 5.2a3 3 0 0 1 0 5.6M17.5 14.2a5 5 0 0 1 3 5.3"/></>,
calc:<><rect x="5" y="2.8" width="14" height="18.4" rx="3"/><rect x="8" y="6" width="8" height="3.6" rx="1"/><path d="M8.6 13.4h.01M12 13.4h.01M15.4 13.4h.01M8.6 17h.01M12 17h.01M15.4 17h.01" strokeWidth="2.2"/></>,
file:<><path d="M14 2.8H7.5a2.5 2.5 0 0 0-2.5 2.5v13.4a2.5 2.5 0 0 0 2.5 2.5h9a2.5 2.5 0 0 0 2.5-2.5V7.8Z"/><path d="M14 2.8v5h5M8.8 13h6.4M8.8 16.6h4.2"/></>,
chart:<><path d="M4 4v14.5A1.5 1.5 0 0 0 5.5 20H20"/><path d="m7.5 15 3.5-4 3 2.6 5-6.1"/></>,
calendar:L417(<><rect x="2.8" y="3.8" width="12.4" height="11.4" rx="2"/><path d="M2.8 7.5h12.4M6.2 2.5v2.6M11.8 2.5v2.6"/></>),
search:<><circle cx="10.8" cy="10.8" r="6.3"/><path d="m15.6 15.6 4.6 4.6"/></>,
settings:L417(<><circle cx="9" cy="9" r="2.3"/><path d="M9 2.6v1.8M9 13.6v1.8M2.6 9h1.8M13.6 9h1.8M4.5 4.5l1.3 1.3M12.2 12.2l1.3 1.3M4.5 13.5l1.3-1.3M12.2 5.8l1.3-1.3"/></>),
arrow:<path d="M5 12h14m-5.5-5.5L19 12l-5.5 5.5"/>,
plus:<path d="M12 5v14M5 12h14"/>,
check:<path d="m5 12.5 4.5 4.5L19 7.5"/>,
chevron:<path d="m9.5 5.5 6.5 6.5-6.5 6.5"/>,
book:<><path d="M12 6.5C10.3 5 7.8 4.3 4 4.5v13c3.8-.2 6.3.5 8 2 1.7-1.5 4.2-2.2 8-2v-13c-3.8-.2-6.3.5-8 2Z"/><path d="M12 6.5v13"/></>,
sun:<><circle cx="12" cy="12" r="4"/><path d="M12 2.8v1.6M12 19.6v1.6M2.8 12h1.6M19.6 12h1.6M5.5 5.5l1.1 1.1M17.4 17.4l1.1 1.1M5.5 18.5l1.1-1.1M17.4 6.6l1.1-1.1"/></>,
shield:<><path d="M12 2.8 19.5 6v5.6c0 4.6-3.2 8-7.5 9.6-4.3-1.6-7.5-5-7.5-9.6V6Z"/><path d="m8.8 12 2.2 2.2 4.3-4.6"/></>,
upload:<><path d="M12 15.5V4m-4.5 4.5L12 4l4.5 4.5"/><path d="M4 15v3.5A2.5 2.5 0 0 0 6.5 21h11a2.5 2.5 0 0 0 2.5-2.5V15"/></>,
close:<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>,
tool:<><path d="M14.7 6.3a4.2 4.2 0 0 0 5.5 5.2l-8.6 8.6a2.4 2.4 0 0 1-3.4-3.4l8.6-8.6"/><path d="m14.7 6.3 2.6-2.6a4.2 4.2 0 0 1 2.9 7.8"/></>,
library:<><rect x="4" y="4" width="4.5" height="16" rx="1.2"/><rect x="10.5" y="4" width="4" height="16" rx="1.2"/><path d="m16.8 5.6 3.4 13.6"/></>,
spark:<><path d="M12 3.5c.6 4.4 2.6 6.4 7 7-4.4.6-6.4 2.6-7 7-.6-4.4-2.6-6.4-7-7 4.4-.6 6.4-2.6 7-7Z"/><path d="M19 3v3M17.5 4.5h3"/></>};
export function Icon({name,size=20}:{name:string;size?:number}){return <svg className="ic" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{ICONS[name]??ICONS.file}</svg>}
function slug(value:string){return value.normalize('NFD').replace(/[\u0300-\u036f]/g,'').toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/^-|-$/g,'')}
export function Button({children,primary=false,...props}:ButtonHTMLAttributes<HTMLButtonElement>&{primary?:boolean}){
  const id=useId(),button=useRef<HTMLButtonElement>(null),tooltip=useRef<HTMLSpanElement>(null);
  const tooltipText=props['aria-label']?(props.title??props['aria-label']):undefined;
  const tooltipId='tooltip-'+slug(id);
  const [position,setPosition]=useState<{anchor:DOMRect;left:number;top:number}|null>(null);
  const show=()=>{if(tooltipText&&!props.disabled&&button.current){const anchor=button.current.getBoundingClientRect();setPosition({anchor,left:anchor.left,top:anchor.bottom+8})}};
  useLayoutEffect(()=>{
    if(!position||!tooltip.current)return;
    const box=tooltip.current.getBoundingClientRect(),a=position.anchor;
    const left=Math.min(Math.max(8,a.left),Math.max(8,window.innerWidth-box.width-8));
    const top=a.top-box.height-8>=8?a.top-box.height-8:Math.min(a.bottom+8,Math.max(8,window.innerHeight-box.height-8));
    if(Math.abs(left-position.left)>.5||Math.abs(top-position.top)>.5)setPosition({...position,left,top});
  },[position]);
  useEffect(()=>{if(!position)return;const close=()=>setPosition(null);window.addEventListener('resize',close);window.addEventListener('scroll',close,true);return()=>{window.removeEventListener('resize',close);window.removeEventListener('scroll',close,true)}},[position]);
  const described=[props['aria-describedby'],tooltipText?tooltipId:undefined].filter(Boolean).join(' ')||undefined;
  return <><button type="button" ref={button} data-od-id={'control-'+slug(props['aria-label']??(typeof children==='string'?children:'button'))+'-'+slug(id)} {...props} title={props.title??props['aria-label']} aria-describedby={described} data-ui-tooltip={tooltipText?tooltipId:undefined} className={'btn '+(primary?'btn-primary ':'btn-ghost ')+(props.className??'')}
    onMouseEnter={e=>{props.onMouseEnter?.(e);show()}} onMouseLeave={e=>{props.onMouseLeave?.(e);setPosition(null)}}
    onFocus={e=>{props.onFocus?.(e);if(document.documentElement.dataset.input!=='pointer'&&e.currentTarget.matches(':focus-visible'))show()}}
    onBlur={e=>{props.onBlur?.(e);setPosition(null)}} onKeyDown={e=>{props.onKeyDown?.(e);if(e.key==='Escape'&&position){e.preventDefault();e.stopPropagation();setPosition(null)}}}>{children}</button>
    {tooltipText&&createPortal(<span ref={tooltip} id={tooltipId} role="tooltip" className="ui-tooltip" hidden={!position} style={position?{left:position.left,top:position.top}:undefined}>{tooltipText}</span>,document.body)}</>;
}
export function Badge({children,tone='neutral'}:{children:ReactNode;tone?:string}){return <span className={'status-tag '+tone}>{children}</span>}
export function Notice({children,tone='review'}:{children:ReactNode;tone?:string}){return <div className={'notice '+tone} role={tone==='error'?'alert':undefined}>{children}</div>}
export function Panel({title,subtitle,actions,children,className=''}:{title:string;subtitle?:string;actions?:ReactNode;children:ReactNode;className?:string}){const id=useId();return <section data-od-id={'panel-'+slug(title)+'-'+slug(id)} data-ui-container className={'card '+className}><header className="card__head"><div><h2 data-od-id={'title-'+slug(title)+'-'+slug(id)} className="card__title">{title}</h2>{subtitle&&<p className="card__hint">{subtitle}</p>}</div>{actions}</header><div className="card__body" data-ui-content>{children}</div></section>}
// Estado vazio com a pasta flutuante do Lucian UX/UI Prompt #440 (MIT, https://lucianlabs.dev/uxui-prompt/#440).
export function Empty({title,description,action}:{title:string;description:string;action?:ReactNode}){return <div className="empty-state ux u440 u440-static"><EmptyFolder/><h3>{title}</h3><p>{description}</p>{action}</div>}
export function Field({label,value,onChange,placeholder='',type='text',required=false,error,hint}:{label:string;value:string;onChange:(s:string)=>void;placeholder?:string;type?:string;required?:boolean;error?:string;hint?:string}){const id=useId(),errorId=id+'-error';return <label data-od-id={'field-'+slug(label)+'-'+slug(id)} className="field"><span>{label}{required&&<b aria-hidden="true"> *</b>}</span><input className="input" aria-label={label} aria-invalid={error?true:undefined} aria-describedby={error?errorId:undefined} value={value} onChange={e=>onChange(e.target.value)} placeholder={placeholder} type={type} required={required}/>{hint&&<small className="field-hint">{hint}</small>}{error&&<span className="field-error" id={errorId} role="alert">{error}</span>}</label>}
export function Select({label,value,onChange,items,required=false,error,hint}:{label:string;value:string;onChange:(s:string)=>void;items:string[];required?:boolean;error?:string;hint?:string}){const id=useId(),errorId=id+'-error';return <label data-od-id={'field-'+slug(label)+'-'+slug(id)} className="field"><span>{label}{required&&<b aria-hidden="true"> *</b>}</span><select className="input" aria-label={label} aria-required={required||undefined} aria-invalid={error?true:undefined} aria-describedby={error?errorId:undefined} value={value} onChange={e=>onChange(e.target.value)}>{items.map(x=><option key={x} value={x}>{({simple:'Simples',compound:'Compostos',simple_interest:'Juros simples',compound_interest:'Valor futuro / juros compostos',present_value:'Valor presente',discount:'Desconto racional',payment:'Prestação periódica',loan_schedule:'Cronograma de amortização',annuity_present_value:'Valor presente de parcelas',annuity_future_value:'Valor futuro de parcelas',equivalent_rate:'Taxa equivalente',rate_from_values:'Taxa implícita entre valores',pdf_detailed:'PDF · relatório detalhado completo',pdf_summary:'PDF · resumo da hipótese',pdf_periods:'PDF · períodos de contribuição',pdf_salaries:'PDF · salários e concomitâncias',pdf_memory:'PDF · memória de cálculo',pdf_transitions:'PDF · regras e transições',pdf_indicators:'PDF · indicadores e pendências',pdf_quality:'PDF · qualidade de segurado',html:'HTML · relatório local',csv:'CSV · planilha nominal',json:'JSON · resumo técnico','42':'42 · Aposentadoria por tempo de contribuição ou programada','41':'41 · Aposentadoria por idade','32':'32 · Aposentadoria por incapacidade permanente','31':'31 · Auxílio por incapacidade temporária',comum:'Comum (sem nexo declarado)',acidente:'Acidente de trabalho',doenca_ocupacional:'Doença ocupacional',doenca_do_trabalho:'Doença do trabalho',doenca_listada:'Doença da lista (isenção de carência)'}as Record<string,string>)[x]??(x||'Selecione...')}</option>)}</select>{hint&&<small className="field-hint">{hint}</small>}{error&&<span className="field-error" id={errorId} role="alert">{error}</span>}</label>}
// A pilha também protege o fundo quando a busca é aberta sobre outro diálogo.
const modalStack:HTMLDivElement[]=[];
let modalBackground:{el:HTMLElement;inert:boolean}[]=[];
function syncModalStack(){modalStack.forEach((dialog,index)=>{const scrim=dialog.parentElement;if(index===modalStack.length-1)scrim?.removeAttribute('inert');else scrim?.setAttribute('inert','')})}
export function Modal({title,subtitle,children,onClose,wide=false}:{title:string;subtitle?:string;children:ReactNode;onClose:()=>void;wide?:boolean}){
  const ref=useRef<HTMLDivElement>(null),id=useId(),{enabled}=useMotionPolicy();
  useEffect(()=>{
    const previous=document.activeElement as HTMLElement|null;
    if(!modalStack.length){modalBackground=Array.from(document.querySelectorAll<HTMLElement>('.app-side,.top-chrome,.app-content,.mobile-dock,.skip-link')).map(el=>({el,inert:el.hasAttribute('inert')}));modalBackground.forEach(({el})=>el.setAttribute('inert',''))}
    const dialog=ref.current!;modalStack.push(dialog);syncModalStack();
    (dialog.querySelector<HTMLElement>('.dialog__body input:not([disabled]),.dialog__body textarea:not([disabled]),.dialog__body select:not([disabled])')??dialog.querySelector<HTMLElement>('.dialog__body button:not([disabled])')??dialog.querySelector<HTMLElement>('button:not([disabled])')??dialog).focus();
    return()=>{
      const index=modalStack.indexOf(dialog);if(index>=0)modalStack.splice(index,1);syncModalStack();
      if(!modalStack.length){modalBackground.forEach(({el,inert})=>{if(!inert)el.removeAttribute('inert')});modalBackground=[];if(previous?.isConnected)previous.focus({preventScroll:true})}
      else {const top=modalStack.at(-1)!;if(previous?.isConnected&&top.contains(previous))previous.focus({preventScroll:true});else top.querySelector<HTMLElement>('button:not([disabled]),input:not([disabled])')?.focus()}
    };
  },[]);
  return createPortal(<div className="dialog-scrim" onMouseDown={e=>{if(e.target===e.currentTarget)onClose()}}><div className="dialog-backdrop" aria-hidden="true"/>
    <motion.div initial={enabled&&document.documentElement.dataset.input==='pointer'?{y:12,scale:0.985}:false} animate={{y:0,scale:1}} transition={enabled?springs.snappy:{duration:0}} ref={ref} tabIndex={-1} data-od-id={'dialog-'+slug(title)} className={'dialog '+(wide?'dialog--wide':'')} role="dialog" aria-modal="true" aria-labelledby={id+'-title'} aria-describedby={subtitle?id+'-description':undefined} onKeyDown={e=>{
      if(e.key==='Escape'){e.preventDefault();e.stopPropagation();onClose()}
      if(e.key==='Tab'){
        const elements=Array.from(ref.current?.querySelectorAll<HTMLElement>('a[href],button:not([disabled]),input:not([disabled]),textarea:not([disabled]),select:not([disabled]),summary,[tabindex]:not([tabindex="-1"])')??[]).filter(el=>!el.closest('[hidden]'));
        const first=elements[0],last=elements.at(-1);
        if(!first){e.preventDefault();ref.current?.focus()}
        else if(e.shiftKey&&(document.activeElement===first||document.activeElement===ref.current)){e.preventDefault();last?.focus()}
        else if(!e.shiftKey&&(document.activeElement===last||document.activeElement===ref.current)){e.preventDefault();first.focus()}
      }
    }}>
      <header className="dialog__head"><div><h2 id={id+'-title'}>{title}</h2>{subtitle&&<p id={id+'-description'}>{subtitle}</p>}</div><Button aria-label="Fechar janela" onClick={onClose}><Icon name="close"/></Button></header>
      <div className="dialog__body">{children}</div>
    </motion.div>
  </div>, document.querySelector('.previdenciar-app') ?? document.body)
}

