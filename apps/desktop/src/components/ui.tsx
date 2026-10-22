import {useEffect,useId,useLayoutEffect,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {motion} from 'motion/react';
import {springs,useMotionPolicy} from './amicro/motion';
import type{ReactNode,ButtonHTMLAttributes}from'react';
export function Icon({name,size=20}:{name:string;size?:number}){const p:Record<string,ReactNode>={motion:<><path d="M3 8h5m-5 4h3m-3 4h5"/><path d="m12 4 8 8-8 8-4-8Z"/></>,menu:<path d="M4 6h16M4 12h16M4 18h16"/>,home:<><path d="m3 10 9-7 9 7v10H3Z"/><path d="M9 20v-7h6v7"/></>,users:<><circle cx="9" cy="7" r="3"/><path d="M3 20v-3a6 6 0 0 1 12 0v3M17 4a3 3 0 0 1 0 6M19 14a5 5 0 0 1 2 6"/></>,calc:<><rect x="5" y="2" width="14" height="20" rx="3"/><path d="M8 7h8M8 12h1m3 0h1m3 0h1M8 16h1m3 0h1m3 0h1"/></>,file:<><path d="M5 2h10l4 4v16H5Z"/><path d="M14 2v5h5M8 12h8M8 16h8"/></>,chart:<><path d="M3 3v18h18M7 16v-5M12 16V7M17 16v-9"/></>,calendar:<><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M7 2v6M17 2v6M3 11h18M7 15h2m4 0h2"/></>,search:<><circle cx="10" cy="10" r="6"/><path d="m15 15 6 6"/></>,settings:<><path d="m10 2-1 3-3 1-3 1 1 4-1 4 3 2 3 1 1 3h4l1-3 3-1 3-2-1-4 1-4-3-1-3-1-1-3Z"/><circle cx="12" cy="12" r="3"/></>,arrow:<path d="M4 12h16m-6-6 6 6-6 6"/>,plus:<path d="M12 4v16M4 12h16"/>,check:<path d="m4 12 5 5L20 6"/>,chevron:<path d="m8 4 8 8-8 8"/>,book:<><path d="M3 3h7l2 3 2-3h7v17h-7l-2 2-2-2H3Z"/><path d="M12 6v16"/></>,sun:<><circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l2 2m10 10 2 2M19 5l-2 2M7 17l-2 2"/></>,shield:<><path d="m12 2 9 4v6c0 5-9 10-9 10S3 17 3 12V6Z"/><path d="m8 12 3 3 5-6"/></>,upload:<><path d="M12 16V3m-5 5 5-5 5 5M3 16v5h18v-5"/></>,close:<path d="m5 5 14 14M19 5 5 19"/>,tool:<><path d="M14 3a6 6 0 0 0-7 8L2 16l6 6 5-8a6 6 0 0 0 8-7l-4 4-4-4Z"/></>};return <svg className="ic" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{p[name]??p.file}</svg>}
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
export function Empty({title,description,action}:{title:string;description:string;action?:ReactNode}){return <div className="empty-state"><span className="empty-emblem"><Icon name="file" size={27}/></span><h3>{title}</h3><p>{description}</p>{action}</div>}
export function Field({label,value,onChange,placeholder='',type='text',required=false}:{label:string;value:string;onChange:(s:string)=>void;placeholder?:string;type?:string;required?:boolean}){const id=useId();return <label data-od-id={'field-'+slug(label)+'-'+slug(id)} className="field"><span>{label}{required&&<b aria-hidden="true"> *</b>}</span><input className="input" aria-label={label} value={value} onChange={e=>onChange(e.target.value)} placeholder={placeholder} type={type} required={required}/></label>}
export function Select({label,value,onChange,items}:{label:string;value:string;onChange:(s:string)=>void;items:string[]}){const id=useId();return <label data-od-id={'field-'+slug(label)+'-'+slug(id)} className="field"><span>{label}</span><select className="input" aria-label={label} value={value} onChange={e=>onChange(e.target.value)}>{items.map(x=><option key={x} value={x}>{({simple:'Simples',compound:'Compostos',simple_interest:'Juros simples',compound_interest:'Valor futuro / juros compostos',present_value:'Valor presente',discount:'Desconto racional',payment:'Prestação periódica',loan_schedule:'Cronograma de amortização',annuity_present_value:'Valor presente de parcelas',annuity_future_value:'Valor futuro de parcelas',equivalent_rate:'Taxa equivalente',rate_from_values:'Taxa implícita entre valores',pdf_detailed:'PDF — relatório detalhado completo',pdf_summary:'PDF — resumo da hipótese',pdf_periods:'PDF — períodos de contribuição',pdf_salaries:'PDF — salários e concomitâncias',pdf_memory:'PDF — memória de cálculo',pdf_transitions:'PDF — regras e transições',pdf_indicators:'PDF — indicadores e pendências',pdf_quality:'PDF — qualidade de segurado',html:'HTML — relatório local',csv:'CSV — planilha nominal',json:'JSON — resumo técnico'}as Record<string,string>)[x]??(x||'Não informado')}</option>)}</select></label>}
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
  </div>, document.querySelector('.previus-app') ?? document.body)
}
