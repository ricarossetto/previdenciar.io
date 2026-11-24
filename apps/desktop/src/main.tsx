import { createRoot } from 'react-dom/client';
import App from './App';
import { WebAuthGate } from './auth-web';
import { ConsentBanner } from './components/ConsentBanner';
import { installErrorReporting } from './web-telemetry';
import './styles/tokens.css';
import './styles/app.css';
import './styles/mesa.css';
import './styles/amicro.css';
import './styles/lucian.css';
import './styles/ai.css';
import './styles/knowledge.css';
import './styles/minuta.css';
import './styles/journey.css';

if (import.meta.env.MODE === 'web') installErrorReporting();

createRoot(document.getElementById('root')!).render(
  import.meta.env.MODE === 'web' ? (
    // O envoltório não desenha caixa (display: contents); só aplica os estilos de botão e campo
    // do app também na tela de entrada, nos termos e no aviso de privacidade.
    <div className="previdenciar-app is-standalone">
      <WebAuthGate />
      <ConsentBanner />
    </div>
  ) : (
    <App />
  ),
);
