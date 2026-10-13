import type { Workspace } from '../../model';
import { blockers } from '../../model';
import { Button, Icon } from '../ui';
import { FadeUp, TiltCard } from './motion';

export function DashboardOverview({ workspace, onStart, onClient, onReports }: {
  workspace: Workspace; onStart: () => void; onClient: () => void; onReports: () => void;
}) {
  const active = workspace.calculations.filter(item => !item.archived);
  const metrics = [
    { label: 'Clientes ativos', value: workspace.clients.filter(item => !item.archived).length, icon: 'users' },
    { label: 'Cálculos registrados', value: active.length, icon: 'calc' },
    { label: 'Revisões pendentes', value: active.filter(item => blockers(item).length > 0).length, icon: 'shield' },
    { label: 'Compromissos abertos', value: workspace.appointments.filter(item => !item.completed).length, icon: 'calendar' },
  ];
  const actions = [
    { number: '01', icon: 'calc', title: 'Organize a hipótese', description: 'Uma entrevista, uma etapa por vez. Sem preencher o que ainda precisa ser confirmado.', label: 'Abrir entrevista', onClick: onStart },
    { number: '02', icon: 'users', title: 'Conheça cada caso', description: 'Identificação, documentos e histórico reunidos em um cadastro local protegido.', label: 'Cadastrar cliente', onClick: onClient },
    { number: '03', icon: 'file', title: 'Leve a memória junto', description: 'Confira os dados e gere os relatórios PDF com o motor real do aplicativo.', label: 'Consultar relatórios', onClick: onReports },
  ];
  return <>
    <FadeUp className="amicro-overview">
      <div className="amicro-overview-copy">
        <div className="amicro-kicker"><span />SEU ESPAÇO DE TRABALHO</div>
        <h2>Seu próximo passo,<br /><em>com clareza.</em></h2>
        <p>Da primeira informação à memória de cálculo.<br className="amicro-desktop-break" /> Tudo no seu ritmo. Tudo no seu computador.</p>
        <div className="amicro-overview-meta"><Icon name="shield" size={16} />Base local · revisão profissional preservada</div>
      </div>
      <div className="amicro-orbit" aria-hidden="true">
        <div className="amicro-orbit-ring amicro-orbit-ring--outer" />
        <div className="amicro-orbit-ring amicro-orbit-ring--inner" />
        <div className="amicro-orbit-core"><Icon name="chart" size={46} /></div>
        <span className="amicro-orbit-node amicro-orbit-node--one"><Icon name="file" size={24} /></span>
        <span className="amicro-orbit-node amicro-orbit-node--two"><Icon name="shield" size={24} /></span>
        <span className="amicro-orbit-node amicro-orbit-node--three"><Icon name="calc" size={24} /></span>
        <span className="amicro-orbit-dot" />
      </div>
    </FadeUp>
    <div className="amicro-metrics" role="group" aria-label="Indicadores da base local">
      {metrics.map((metric, index) => <FadeUp key={metric.label} delay={index * 0.035} className="amicro-metric">
        <span className="amicro-metric-label"><Icon name={metric.icon} size={17} />{metric.label}</span>
        <strong data-ui-number>{metric.value}</strong><span className="amicro-metric-note">Na sua base local</span>
      </FadeUp>)}
    </div>
    <div className="amicro-section-caption"><h2>Um fluxo, do início ao relatório</h2><span>ESCOLHA POR ONDE COMEÇAR</span></div>
    <div className="amicro-action-grid">{actions.map((action, index) => <TiltCard key={action.number} index={index}>
      <div className="amicro-action-top"><span className="amicro-action-icon"><Icon name={action.icon} size={23} /></span><span className="amicro-action-number">{action.number}</span></div>
      <h3>{action.title}</h3><p>{action.description}</p>
      <Button onClick={action.onClick}>{action.label}<Icon name="arrow" size={16} /></Button>
    </TiltCard>)}</div>
  </>;
}
