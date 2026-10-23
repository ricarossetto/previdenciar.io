import type {AssistantConsideration, ConsiderationFix} from '../assistantReview';
import {Badge, Icon, Notice} from './ui';

const count=(value:number,singular:string,plural:string)=>value+' '+(value===1?singular:plural);
export function AssistantConsiderationPanel({consideration, onFix}:{consideration:AssistantConsideration;onFix?:(fix:ConsiderationFix)=>void}) {
  const {imported,adjusted,used,issues,sourceNotes,effect,conclusion,ready,execution} = consideration;
  const decisive = issues.filter(issue => issue.decisive);
  const reviewOnly = issues.filter(issue => !issue.decisive);
  return <section className="consideration-panel" aria-label="Dados importados, ajustados e usados no cálculo">
    <header className="consideration-panel__heading">
      <div><span className="eyebrow">Rastreabilidade</span><h3>O que entra no cálculo</h3></div>
      <Badge tone={ready?'good':'review'}>{ready?'Pronto para o motor':decisive.length+' pendência(s) decisiva(s)'}</Badge>
    </header>
    <div className="consideration-flow">
      <div className="consideration-card">
        <span>Importado da fonte</span>
        <strong>{imported.present?count(imported.links,'vínculo','vínculos')+' · '+count(imported.entries,'lançamento','lançamentos'):'Sem CNIS aplicado'}</strong>
        <small>{imported.present?(count(imported.months,'competência','competências')+' · '+count(imported.warnings,'aviso','avisos')+' de leitura'):'O cálculo pode seguir com registros manuais.'}</small>
      </div>
      <span className="consideration-arrow" aria-hidden="true"><Icon name="arrow" size={16}/></span>
      <div className="consideration-card">
        <span>Ajustado pelo operador</span>
        <strong>{count(adjusted.selectedLinks,'vínculo','vínculos')+' · '+count(adjusted.salaries,'remuneração','remunerações')}</strong>
        <small>{adjusted.excludedLinks?'+'+count(adjusted.excludedLinks,'vínculo excluído','vínculos excluídos')+' · ':''}{adjusted.contributionApplied?'livro aplicado':'livro '+(adjusted.contributionRecords?'pendente':'não usado')}</small>
      </div>
      <span className="consideration-arrow" aria-hidden="true"><Icon name="arrow" size={16}/></span>
      <div className={'consideration-card '+(ready?'is-ready':'is-blocked')}>
        <span>{execution==='calculated'?'Execução registrada':'Preparado para execução'}</span>
        <strong>{execution==='blocked'?'Última execução bloqueada':ready?count(used.links,'vínculo','vínculos')+' · '+count(used.salaries,'competência','competências'):'Entrada com pendências'}</strong>
        <small>{execution==='calculated'?'Resultado experimental disponível; confira a memória do motor.':execution==='blocked'?'Nenhum resultado financeiro foi produzido.':ready?'Somente linhas revisadas; a fonte não é reescrita.':'Revise os dados antes de enviar a hipótese ao motor.'}</small>
      </div>
    </div>
    <p className="consideration-effect">{effect}</p>
    {ready?<Notice tone="ok"><strong>Hipótese revisada.</strong> {conclusion}</Notice>:<Notice><strong>Nenhum resultado financeiro ainda.</strong> {conclusion}</Notice>}
    {decisive.length>0&&<div className="consideration-blockers"><h4>Pendências que impedem calcular</h4><ul>{decisive.map(issue=><li key={issue.code}><Badge tone="review">Decisiva</Badge><span>{issue.label}</span>{onFix&&<button type="button" className="link-button" aria-label={'Corrigir: '+issue.label} onClick={()=>onFix(issue.fix)}>Corrigir</button>}</li>)}</ul></div>}
    {reviewOnly.length>0&&<details className="details"><summary>Outras conferências · {reviewOnly.length}</summary><ul className="review-list">{reviewOnly.map(issue=><li key={issue.code}>{issue.label}</li>)}</ul></details>}
    {sourceNotes.length>0&&<details className="details"><summary>Pendências da fonte que não entram no cálculo · {sourceNotes.length}</summary><ul className="review-list">{sourceNotes.map(note=><li key={note.code}>{note.label}</li>)}</ul><p className="form-note">A fonte CNIS permanece preservada. Excluí-la do motor não corrige o documento nem comprova que a exclusão esteja certa.</p></details>}
  </section>;
}
