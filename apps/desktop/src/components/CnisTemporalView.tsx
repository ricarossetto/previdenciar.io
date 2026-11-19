import type { Cnis, ManualLink } from '../model';
import { centavosMoney } from '../model';
import { competenceLayers, confirmedRangesOf, monthKey, pbcWindow, temporalIssues, temporalSummary, type CompetenceLayer } from '../temporal';
import { Notice, Panel } from './ui';

const LIMIT = 120;

function situation(layer: CompetenceLayer): string {
  if (layer.relativeToDib === 'after') return 'Fora do PBC: posterior a DIB (mantida na fonte)';
  if (layer.salaryStatus === 'missing') return 'Sem remuneração: desconhecido, não zero';
  if (layer.salaryStatus === 'dib_month') return 'Mês da DIB: decisão pendente';
  if (layer.salaryStatus === 'zero') return 'Valor zero na origem: conferir';
  if (layer.pbcStatus === 'excluded_below_minimum') return 'Dispensada por valor mínimo';
  if (layer.pbcStatus === 'discarded') return 'Descartada do cálculo: mantida na fonte';
  if (layer.periodStatus === 'unconfirmed') return 'Período sem confirmacao';
  if (layer.periodStatus === 'missing') return 'Sem período correspondente';
  return 'Dentro do PBC';
}
function money(layer: CompetenceLayer): string {
  return layer.amountCentavos === null ? 'Sem dado' : centavosMoney(layer.amountCentavos);
}
function Row({ layer }: { layer: CompetenceLayer }) {
  return <tr data-competence={layer.competence} data-pbc={layer.pbcStatus}>
    <td>{layer.competence}</td>
    <td className='num'>{money(layer)}</td>
    <td>{situation(layer)}</td>
    <td>{layer.sourcePages.length ? 'CNIS p. ' + layer.sourcePages.join(', ') : 'Sem página de origem'} · período {layer.periodStatus === 'confirmed' ? 'confirmado' : layer.periodStatus === 'unknown' ? 'não apurado' : 'pendente'}</td>
  </tr>;
}
function Table({ title, layers, open = false }: { title: string; layers: readonly CompetenceLayer[]; open?: boolean }) {
  const shown = layers.slice(0, LIMIT);
  return <details className='details' open={open}>
    <summary><strong>{title}</strong> - {layers.length}</summary>
    <div className='table-wrap'><table className='data-table'>
      <thead><tr><th>Competência</th><th className='num'>Remuneração</th><th>Situação</th><th>Origem / período</th></tr></thead>
      <tbody>{shown.map(layer => <Row key={layer.competence} layer={layer} />)}</tbody>
    </table></div>
    {layers.length > shown.length && <p className='form-note'>Exibindo {shown.length} de {layers.length}. A fonte integral continua nas abas Períodos e Remunerações.</p>}
    {!layers.length && <p className='form-note'>Nenhuma competência nesta faixa.</p>}
  </details>;
}

export interface CnisTemporalViewProps { cnis: Cnis; dib: string; confirmedLinks?: readonly ManualLink[] }

/** Visão temporal derivada do CNIS: nunca apaga a fonte e limita o recorte de cálculo a DIB/PBC. */
export function CnisTemporalView({ cnis, dib, confirmedLinks }: CnisTemporalViewProps) {
  const window = pbcWindow(dib);
  const dibKey = monthKey(dib);
  const confirmedRanges = confirmedRangesOf(Array.isArray(confirmedLinks) ? confirmedLinks : []);
  const hasConfirmedLinks = confirmedRanges.length > 0;
  const layers = competenceLayers(cnis, dib, hasConfirmedLinks ? { confirmedRanges } : {});
  const summary = temporalSummary(layers, window);
  const issues = temporalIssues(layers).filter(group => group.key !== 'sem_periodo' || hasConfirmedLinks);
  const postDib = layers.filter(layer => layer.relativeToDib === 'after');
  const missing = layers.filter(layer => layer.salaryStatus === 'missing');
  const calculationLayers = layers.filter(layer => layer.inPbcWindow && layer.amountCentavos !== null && layer.relativeToDib !== 'after');
  const beforeWindow = layers.filter(layer => layer.pbcStatus === 'before_window' && layer.relativeToDib !== 'after');
  return <Panel title='Visão temporal do CNIS' subtitle='Camada derivada e aditiva: a fonte importada não é apagada nem reescrita; o recorte de cálculo termina na DIB/PBC.'>
    <div className='result-strip' data-testid='temporal-summary'>
      <div><span>Janela do PBC</span><strong>{window ? window.from + ' a ' + window.to : (dibKey ? 'Anterior a 07/1994 (fora do PBC)' : 'DIB não informada')}</strong></div>
      <div><span>Dentro do PBC</span><strong>{summary.inPbc}</strong></div>
      <div><span>Posterior a DIB (fora do PBC)</span><strong>{summary.postDib}</strong></div>
      <div><span>Sem remuneração (desconhecida)</span><strong>{summary.missingSalary}</strong></div>
      <div><span>Sem período confirmado</span><strong data-testid='temporal-unconfirmed'>{hasConfirmedLinks ? summary.unconfirmed : 'Não apurado'}</strong></div>
    </div>
    <Notice>Posterior a DIB permanece na fonte e fica fora do PBC. Competência sem período confirmado fica pendente; competência sem remuneração continua desconhecida, nunca zero. As famílias 'posterior a DIB' e 'sem período confirmado' são dimensões independentes: não devem ser somadas.</Notice>
    <p className='form-note' data-testid='temporal-window-note'>A janela do PBC tem {summary.windowMonths || 0} meses; {summary.withSalaryInPbc} competência(s) com remuneração no recorte e {summary.missingSalary} sem remuneração lançada. Esta contagem deriva das competências e dos trechos de vínculo presentes na origem, não de uma varredura de todos os meses da janela: mês sem lançamento exige conferência; não é lacuna automática nem salário zero.</p>
    {issues.length > 0 && <div data-testid='temporal-issues'>
      <h3 style={{ fontSize: 14 }}>Pendências agrupadas e próxima ação</h3>
      <ul className='review-list'>{issues.map(group => <li key={group.key} data-group={group.key}>
        <strong>{group.title}</strong> - {group.count}
        <div className='secondary-line'>{group.nextAction}</div>
      </li>)}</ul>
    </div>}
    <Table title='Posterior a DIB: fora do PBC, preservada na fonte' layers={postDib} open={postDib.length > 0 && postDib.length <= 40} />
    <Table title='Sem remuneração lancada: valor desconhecido, não zero' layers={missing} />
    <Table title='Recorte de cálculo: competências com remuneração dentro do PBC' layers={calculationLayers} />
    <Table title='Anterior a janela do PBC (antes de 1994-07)' layers={beforeWindow} />
    <p className='form-note'>Fonte preservada: {cnis.salary_entries.length} lançamentos, {cnis.links.length} vínculos e {Object.keys(cnis.salary_months).length} competências. Esta visão não altera o documento importado.</p>
  </Panel>;
}
