import type { Workspace } from '../../model';
import { blockers } from '../../model';
import { Icon } from '../ui';

/**
 * Entrada da tela inicial: três ações curtas, sem métricas decorativas.
 * A decisão de produto é do advogado; a interface só mostra o próximo passo.
 */
export function DashboardOverview({ workspace, onStart, onClient, onReports }: {
  workspace: Workspace; onStart: () => void; onClient: () => void; onReports: () => void;
}) {
  const active = workspace.calculations.filter(item => !item.archived);
  const pending = active.filter(item => blockers(item).length > 0).length;
  return <section className="home-next" aria-labelledby="home-next-title">
    <header className="home-next-head">
      <h2 id="home-next-title">O que você quer fazer agora?</h2>
    </header>
    <div className="home-next-actions">
      <button type="button" className="home-next-action home-next-action--primary" onClick={onStart}>
        <span className="home-next-icon"><Icon name="calc" size={22} /></span>
        <span className="home-next-copy"><strong>Novo cálculo</strong><small>Abrir a conferência em sete passos.</small></span>
        <Icon name="arrow" size={17} />
      </button>
      <button type="button" className="home-next-action" onClick={onClient}>
        <span className="home-next-icon"><Icon name="users" size={22} /></span>
        <span className="home-next-copy"><strong>Cadastrar cliente</strong><small>Reunir identificação e documentos.</small></span>
        <Icon name="arrow" size={17} />
      </button>
      <button type="button" className="home-next-action" onClick={onReports}>
        <span className="home-next-icon"><Icon name="file" size={22} /></span>
        <span className="home-next-copy"><strong>Ver relatórios</strong><small>Consultar os PDFs já gerados.</small></span>
        <Icon name="arrow" size={17} />
      </button>
    </div>
    {pending > 0 && <p className="home-next-note" role="status">{pending} cálculo(s) com pendência de conferência. <button type="button" className="link-button" onClick={() => onReports()}>Começar pela revisão</button></p>}
    {pending === 0 && active.length > 0 && <p className="home-next-note">Nenhuma pendência aberta. Você pode continuar o cálculo ou emitir o relatório.</p>}
  </section>;
}
