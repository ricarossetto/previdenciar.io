import type { Cnis, ManualLink } from '../model';
import { centavosMoney } from '../model';
import { competenceLayers, confirmedRangesOf, pbcWindow, temporalIssues, temporalSummary, type CompetenceLayer } from '../temporal';
import { Notice, Panel } from './ui';

const LIMIT = 120;

function situation(layer: CompetenceLayer): string {
  if (layer.relativeToDib === 'after') return 'Fora do PBC: posterior a DIB (mantida na fonte)';
  if (layer.salaryStatus === 'missing') return 'Sem remuneracao: desconhecido, nao zero';
  if (layer.salaryStatus === 'dib_month') return 'Mes da DIB: decisao pendente';
  if (layer.salaryStatus === 'zero') return 'Valor zero na origem: conferir';
  if (layer.pbcStatus === 'excluded_below_minimum') return 'Dispensada por valor minimo';
  if (layer.pbcStatus === 'discarded') return 'Descartada do calculo: mantida na fonte';
  if (layer.periodStatus === 'unconfirmed') return 'Periodo sem confirmacao';
  if (layer.periodStatus === 'missing') return 'Sem periodo correspondente';
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
    <td>{layer.sourcePages.length ? 'Paginas ' + layer.sourcePages.join(', ') : 'Sem pagina de origem'}</td>
  </tr>;
}
function Table({ title, layers, open = false }: { title: string; layers: readonly CompetenceLayer[]; open?: boolean }) {
  const shown = layers.slice(0, LIMIT);
  return <details className='details' open={open}>
    <summary><strong>{title}</strong> - {layers.length}</summary>
    <div className='table-wrap'><table className='data-table'>
      <thead><tr><th>Competencia</th><th className='num'>Remuneracao</th><th>Situacao</th><th>Origem</th></tr></thead>
      <tbody>{shown.map(layer => <Row key={layer.competence} layer={layer} />)}</tbody>
    </table></div>
    {layers.length > shown.length && <p className='form-note'>Exibindo {shown.length} de {layers.length}. A fonte integral continua nas abas Periodos e Remuneracoes.</p>}
    {!layers.length && <p className='form-note'>Nenhuma competencia nesta faixa.</p>}
  </details>;
}

export interface CnisTemporalViewProps { cnis: Cnis; dib: string; confirmedLinks?: readonly ManualLink[] }

/** Visao temporal derivada do CNIS: nunca apaga a fonte e limita o recorte de calculo a DIB/PBC. */
export function CnisTemporalView({ cnis, dib, confirmedLinks }: CnisTemporalViewProps) {
  const window = pbcWindow(dib);
  const hasConfirmedLinks = Array.isArray(confirmedLinks) && confirmedLinks.length > 0;
  const layers = competenceLayers(cnis, dib, confirmedLinks ? { confirmedRanges: confirmedRangesOf(confirmedLinks) } : {});
  const summary = temporalSummary(layers, window);
  const issues = temporalIssues(layers).filter(group => group.key !== 'sem_periodo' || hasConfirmedLinks);
  const postDib = layers.filter(layer => layer.relativeToDib === 'after');
  const missing = layers.filter(layer => layer.salaryStatus === 'missing');
  const calculationLayers = layers.filter(layer => layer.inPbcWindow);
  const beforeWindow = layers.filter(layer => layer.pbcStatus === 'before_window' && layer.relativeToDib !== 'after');
  return <Panel title='Visao temporal do CNIS' subtitle='Camada derivada e aditiva: a fonte importada nao e apagada nem reescrita; o recorte de calculo termina na DIB/PBC.'>
    <div className='result-strip' data-testid='temporal-summary'>
      <div><span>Janela do PBC</span><strong>{window ? window.from + ' a ' + window.to : (dib && /^\d{4}-\d{2}/.test(dib) ? 'Anterior a 07/1994 (fora do PBC)' : 'DIB nao informada')}</strong></div>
      <div><span>Dentro do PBC</span><strong>{summary.inPbc}</strong></div>
      <div><span>Posterior a DIB (fora do PBC)</span><strong>{summary.postDib}</strong></div>
      <div><span>Sem remuneracao (desconhecida)</span><strong>{summary.missingSalary}</strong></div>
      <div><span>Sem periodo confirmado</span><strong data-testid='temporal-unconfirmed'>{hasConfirmedLinks ? summary.unconfirmed : 'Nao apurado'}</strong></div>
    </div>
    <Notice>Posterior a DIB permanece na fonte e fica fora do PBC. Competencia sem periodo confirmado fica pendente; competencia sem remuneracao continua desconhecida, nunca zero.</Notice>
    {issues.length > 0 && <div data-testid='temporal-issues'>
      <h3 style={{ fontSize: 14 }}>Pendencias agrupadas e proxima acao</h3>
      <ul className='review-list'>{issues.map(group => <li key={group.key} data-group={group.key}>
        <strong>{group.title}</strong> - {group.count}
        <div className='secondary-line'>{group.nextAction}</div>
      </li>)}</ul>
    </div>}
    <Table title='Posterior a DIB: fora do PBC, preservada na fonte' layers={postDib} open={postDib.length > 0 && postDib.length <= 40} />
    <Table title='Sem remuneracao lancada: valor desconhecido, nao zero' layers={missing} />
    <Table title='Recorte de calculo: dentro da janela do PBC' layers={calculationLayers} />
    <Table title='Anterior a janela do PBC (antes de 1994-07)' layers={beforeWindow} />
    <p className='form-note'>Fonte preservada: {cnis.salary_entries.length} lancamentos, {cnis.links.length} vinculos e {Object.keys(cnis.salary_months).length} competencias. Esta visao nao altera o documento importado.</p>
  </Panel>;
}
