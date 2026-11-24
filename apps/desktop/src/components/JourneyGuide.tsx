import type { Calculation } from '../model';
import {
  JOURNEY_TITLES,
  journeyLayers,
  journeyPendencies,
  journeyStage,
  journeyTechnical,
  primaryJourneyAction,
  type JourneyLayerState,
} from '../journey';
import { Badge, Icon } from './ui';

const LAYER_ICON: Record<'importado' | 'confirmado' | 'calculado', string> = { importado: 'file', confirmado: 'check', calculado: 'calc' };
const LAYER_TONE: Record<JourneyLayerState, string> = { vazio: 'neutral', parcial: 'review', pronto: 'good' };

/// Separação visual entre o que veio do documento, o que o operador confirmou e o
/// que o motor calculou. Os três estados usam os mesmos limites em qualquer tela.
export function JourneyLayers({ draft, label = 'Dados da hipótese' }: { draft: Calculation; label?: string }) {
  const layers = journeyLayers(draft);
  return <section className="journey-layers" aria-label={label}>
    {layers.map(layer => <article key={layer.id} className={'journey-layer is-' + layer.state} data-layer={layer.id}>
      <header>
        <span className="journey-layer__icon"><Icon name={LAYER_ICON[layer.id]} size={16} /></span>
        <h3>{layer.title}</h3>
        <Badge tone={LAYER_TONE[layer.state]}>{layer.stateLabel}</Badge>
      </header>
      <p>{layer.summary}</p>
      <dl className="journey-layer__metrics">
        {layer.metrics.map(metric => <div key={metric.label}><dt>{metric.label}</dt><dd>{metric.value}</dd></div>)}
      </dl>
    </article>)}
  </section>;
}

/// Pendências agrupadas pela próxima ação real: cada grupo leva à etapa que
/// resolve o item, em vez de listar dezenas de mensagens soltas.
export function JourneyPendencies({ draft, onStep }: { draft: Calculation; onStep: (step: number) => void }) {
  const groups = journeyPendencies(draft);
  if (!groups.length) return <section className="journey-pendencies is-empty" aria-label="Pendências por próxima ação">
    <h3>Pendências por próxima ação</h3>
    <p>Nenhuma pendência de dados identificada. A revisão profissional e a execução do motor continuam necessárias.</p>
  </section>;
  const total = groups.reduce((sum, group) => sum + group.items.length, 0);
  return <section className="journey-pendencies" aria-label="Pendências por próxima ação">
    <header><h3>Pendências por próxima ação</h3><p>{total} item(ns) de conferência em {groups.length} frente(s).</p></header>
    <ul>{groups.map(group => <li key={group.action}>
      <button type="button" className="journey-pendency" onClick={() => onStep(group.step)} aria-label={'Abrir ' + JOURNEY_TITLES[group.step] + ': ' + group.label}>
        <span className="journey-pendency__copy">
          <strong>{group.label}</strong>
          <small>{group.hint}</small>
          <em>{group.items.slice(0, 2).join(' · ')}{group.items.length > 2 ? ' +' + (group.items.length - 2) : ''}</em>
        </span>
        <span className="journey-pendency__count" aria-hidden="true">{group.items.length}</span>
        <Icon name="arrow" size={15} />
      </button>
    </li>)}</ul>
  </section>;
}

/// Indicador da etapa atual com a ação principal e os detalhes técnicos recolhidos.
/// Usado pelo editor; o assistente guiado mantém a própria barra de passos.
export function JourneyGuide({ draft, step, onStep }: {
  draft: Calculation;
  step: number;
  onStep: (step: number) => void;
}) {
  const stage = journeyStage(step), action = primaryJourneyAction(draft, step), technical = journeyTechnical(draft);
  return <section className="journey-guide" aria-label="Etapa atual do cálculo" data-od-id="journey-guide">
    <header className="journey-guide__head">
      <div className="journey-guide__stage">
        <span className="eyebrow">Etapa atual · {stage.index + 1} de {JOURNEY_TITLES.length}</span>
        <h2>{stage.title}</h2>
        <p>{action.hint}</p>
      </div>

    </header>
    <JourneyLayers draft={draft} />
    <JourneyPendencies draft={draft} onStep={onStep} />
    <details className="details journey-guide__tech">
      <summary>Detalhes técnicos da origem e do cálculo</summary>
      <dl>{technical.map(line => <div key={line.label}><dt>{line.label}</dt><dd>{line.value}</dd></div>)}</dl>
    </details>
  </section>;
}
