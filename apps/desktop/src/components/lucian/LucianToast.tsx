/* Lucian UX/UI Prompt #435 · MIT License (c) 2026 Seokwoooo · https://lucianlabs.dev/uxui-prompt/#435 */
// Toast único do Previdenciar: sobe com mola, o check se desenha e a barra de tempo corre.
// Mantém role status/alert do toast anterior. Erros não somem sozinhos e não têm barra de tempo.
import { useState } from 'react';

export function LucianToast({
  message,
  error,
  life = 6000,
  onClose,
}: {
  message: string;
  error: boolean;
  life?: number;
  onClose: () => void;
}) {
  const [paused, setPaused] = useState(false);
  return (
    <div className="ux u435" data-od-id="toast">
      <section
        className={'u435-rg' + (paused ? ' is-paused' : '')}
        aria-label="Notificações"
        onPointerEnter={() => setPaused(true)}
        onPointerLeave={() => setPaused(false)}
      >
        <div className="u435-t is-in" role={error ? 'alert' : 'status'}>
          <span className="u435-ic" aria-hidden="true">
            {error ? (
              <span className="er">
                <svg viewBox="0 0 14 14"><path d="M7 3.5v4M7 10.2v.1" /></svg>
              </span>
            ) : (
              <span className="ok">
                <svg viewBox="0 0 14 14"><path d="M3 7.4l2.6 2.6L11 4.4" /></svg>
              </span>
            )}
          </span>
          <span className="u435-tx">
            <b>{message}</b>
          </span>
          <button type="button" className="u435-x" aria-label="Fechar mensagem" onClick={onClose}>
            <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" /></svg>
          </button>
          {!error && (
            <span className="u435-pb" aria-hidden="true">
              <i style={{ animationDuration: life + 'ms' }} />
            </span>
          )}
        </div>
      </section>
    </div>
  );
}
