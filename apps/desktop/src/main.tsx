import { createRoot } from 'react-dom/client';
import App from './App';
import { WebAuthGate } from './auth-web';
import './styles/tokens.css';
import './styles/app.css';
import './styles/mesa.css';
import './styles/amicro.css';
import './styles/ai.css';
import './styles/knowledge.css';
import './styles/minuta.css';
import './styles/journey.css';

createRoot(document.getElementById('root')!).render(import.meta.env.MODE === 'web' ? <WebAuthGate /> : <App />);
