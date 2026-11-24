import { useState } from 'react';
import { getConsent, setConsent } from '../web-telemetry';
import { Button } from './ui';

/** Aviso de privacidade do site. Sem resposta, nada é contado. */
export function ConsentBanner() {
  const [open, setOpen] = useState(() => getConsent() === null);
  if (!open) return null;
  const choose = (value: 'granted' | 'denied') => {
    setConsent(value);
    setOpen(false);
  };
  return (
    <section className="consent-banner" role="region" aria-label="Aviso de privacidade">
      <p>
        Este site não usa cookies de publicidade. Guardamos no seu navegador só o necessário para manter você conectado.
        Com a sua permissão, também contamos de forma anônima quais telas são abertas, para melhorar o produto.{' '}
        <a href="/privacidade.html" target="_blank" rel="noreferrer">Política de Privacidade</a>
      </p>
      <div className="consent-banner__actions">
        <Button onClick={() => choose('denied')}>Recusar contagem</Button>
        <Button primary onClick={() => choose('granted')}>Aceitar</Button>
      </div>
    </section>
  );
}
