import type { Cnis } from '../model';
import { buildCnisTimeline, type TimelineMonthState } from '../timeline';
import { Badge } from './ui';

const STATE_LABEL: Record<TimelineMonthState, string> = {
  lancamento: 'lançamento importado',
  lacuna: 'sem lançamento na origem',
  posDib: 'no mês da DIB ou depois, fora do PBC',
  prePbc: 'antes de 07/1994, fora do recorte do PBC',
  foraDoIntervalo: 'fora do intervalo da fonte',
};

/// Régua temporal da origem CNIS. A fonte é sempre aditiva: competências lidas,
/// janela do PBC, lacunas e períodos posteriores à DIB aparecem no mesmo lugar.
/// Ausência de lançamento é exibida como ausência, nunca como salário zero.
export function CnisTimeline({ cnis, dib }: { cnis: Cnis; dib?: string | null }) {
  const model = buildCnisTimeline(cnis, dib ?? null);
  if (!model.firstCompetence) return <section className="cnis-timeline" aria-label="Visão temporal da origem CNIS">
    <h3>Visão temporal da origem</h3>
    <p className="form-note">Nenhuma competência foi extraída do documento. Ausência de lançamento não é salário zero.</p>
  </section>;
  return <section className="cnis-timeline" aria-label="Visão temporal da origem CNIS">
    <header className="cnis-timeline__head">
      <div>
        <h3>Visão temporal da origem</h3>
        <p>Uma linha por ano; cada coluna é uma competência. A lacuna é ausência na origem; não é salário zero.</p>
      </div>
      <Badge tone={model.counts.lacunas ? 'review' : 'good'}>{model.counts.lancamentos} com lançamento · {model.counts.lacunas} lacunas</Badge>
    </header>
    <ol className="cnis-timeline__years" aria-label="Competências do CNIS por ano">
      {model.years.map(year => <li key={year.year} className="cnis-timeline__year">
        <span className="cnis-timeline__year-label">{year.year}</span>
        <span className="cnis-timeline__months">
          {year.months.map(month => <span
            key={month.competence}
            className={'cnis-timeline__month is-' + month.state}
            title={month.competence + ' · ' + STATE_LABEL[month.state]}
            aria-label={month.competence + ': ' + STATE_LABEL[month.state]}
          />)}
        </span>
      </li>)}
    </ol>
    <dl className="cnis-timeline__counts">
      <div><dt>No recorte do PBC</dt><dd>{model.counts.lancamentos}</dd></div>
      <div><dt>Lacunas na origem</dt><dd>{model.counts.lacunas}</dd></div>
      <div><dt>Antes de 07/1994</dt><dd>{model.counts.prePbc}</dd></div>
      <div><dt>No mês da DIB ou depois</dt><dd>{model.counts.posDib}</dd></div>
      <div><dt>Janela exibida</dt><dd>{model.rangeStart} a {model.rangeEnd}</dd></div>
      <div><dt>Recorte do PBC termina</dt><dd>{model.pbcEnd ?? 'DIB não informada'}</dd></div>
    </dl>
    <p className="form-note">Períodos depois da DIB continuam visíveis como origem, mas ficam fora da janela da média. O descarte de competências é decisão do motor e não é aplicado nesta tela; datas finais ausentes na origem não são presumidas.</p>
  </section>;
}
