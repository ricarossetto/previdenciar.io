/* Lucian UX/UI Prompt #438 · MIT License (c) 2026 Seokwoooo · https://lucianlabs.dev/uxui-prompt/#438 */
// Botão que encolhe em círculo e gira enquanto o motor calcula; se o resultado sair, vira check verde e volta.
// O estado vem de fora (busy/success), não de temporizador fictício.
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';

type S = 'idle' | 'busy' | 'ok';

export function BusyButton({
  busy,
  success,
  disabled,
  onClick,
  children,
  busyLabel = 'Calculando',
  okLabel = 'Cálculo concluído',
}: {
  busy: boolean;
  success: boolean;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
  busyLabel?: string;
  okLabel?: string;
}) {
  const ref = useRef<HTMLButtonElement>(null);
  const label = useRef<HTMLSpanElement>(null);
  const [s, setS] = useState<S>(busy ? 'busy' : 'idle');
  const [down, setDown] = useState(false);
  const [grow, setGrow] = useState(false);
  const full = useRef(0);
  const wasBusy = useRef(busy);

  useLayoutEffect(() => {
    if (s !== 'idle' || !label.current) return;
    const w = Math.ceil(label.current.getBoundingClientRect().width);
    if (w) full.current = w;
  });

  useEffect(() => {
    const timers: number[] = [];
    if (busy && !wasBusy.current) setS('busy');
    if (!busy && wasBusy.current) {
      if (success) {
        setS('ok');
        const rm = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
        timers.push(window.setTimeout(() => {
          setGrow(true);
          setS('idle');
          timers.push(window.setTimeout(() => setGrow(false), rm ? 0 : 700));
        }, rm ? 600 : 1000));
      } else {
        setGrow(true);
        setS('idle');
        timers.push(window.setTimeout(() => setGrow(false), 700));
      }
    }
    wasBusy.current = busy;
    return () => timers.forEach(t => clearTimeout(t));
  }, [busy, success]);

  const width = s === 'idle' ? (full.current ? full.current + 'px' : 'auto') : '48px';
  return (
    <span className="ux u438 u438-inline">
      <button
        ref={ref}
        type="button"
        className={'u438-b' + (down ? ' is-down' : '') + (grow ? ' is-grow' : '')}
        data-s={s}
        style={{ ['--w' as string]: width }}
        disabled={disabled && s === 'idle'}
        aria-busy={s === 'busy' || undefined}
        aria-disabled={s !== 'idle' || undefined}
        onPointerDown={e => { if (e.button === 0 && s === 'idle') setDown(true); }}
        onPointerUp={() => setDown(false)}
        onPointerLeave={() => setDown(false)}
        onClick={() => { if (s === 'idle') onClick(); }}
      >
        <span ref={label} className="u438-l">{children}</span>
        <i className="u438-sp" aria-hidden="true" />
        <svg className="u438-ck" viewBox="0 0 24 24" aria-hidden="true"><path d="m5.5 12.5 4.5 4.5 8.5-9.5" /></svg>
        <i className="u438-ring" aria-hidden="true" />
      </button>
      <span className="u438-live" role="status">{s === 'busy' ? busyLabel : s === 'ok' ? okLabel : ''}</span>
    </span>
  );
}
