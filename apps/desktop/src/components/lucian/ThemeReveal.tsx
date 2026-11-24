/* Lucian UX/UI Prompt #219 · MIT License (c) 2026 Seokwoooo · https://lucianlabs.dev/uxui-prompt/#219 */
// Troca de tema com revelação circular a partir do botão e ícone que gira de sol para lua.
// Adaptação: no lugar das duas cópias da página do exemplo, usa a View Transitions API sobre o app inteiro.

type ViewTransitionDoc = Document & { startViewTransition?: (cb: () => void) => unknown };

export function revealTheme(button: HTMLElement, apply: () => void) {
  const doc = document as ViewTransitionDoc;
  const reduce = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  if (!doc.startViewTransition || reduce) {
    apply();
    return;
  }
  const b = button.getBoundingClientRect();
  const cx = b.left + b.width / 2;
  const cy = b.top + b.height / 2;
  const r = Math.hypot(Math.max(cx, innerWidth - cx), Math.max(cy, innerHeight - cy));
  const root = document.documentElement.style;
  root.setProperty('--u219-cx', cx.toFixed(0) + 'px');
  root.setProperty('--u219-cy', cy.toFixed(0) + 'px');
  root.setProperty('--u219-r', Math.ceil(r) + 'px');
  doc.startViewTransition(() => apply());
}

export function SunMoon({ dark }: { dark: boolean }) {
  return (
    <span className={'u219-tg' + (dark ? ' is-dark' : '')} aria-hidden="true">
      <svg className="u219-sun" viewBox="0 0 16 16">
        <circle cx="8" cy="8" r="3" fill="currentColor" />
        <g stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
          <path d="M8 1.5v1.4M8 13.1v1.4M1.5 8h1.4M13.1 8h1.4M3.4 3.4l1 1M11.6 11.6l1 1M3.4 12.6l1-1M11.6 4.4l1-1" />
        </g>
      </svg>
      <svg className="u219-moon" viewBox="0 0 16 16">
        <path d="M13.5 9.8A5.6 5.6 0 0 1 6.2 2.5a5.6 5.6 0 1 0 7.3 7.3z" fill="currentColor" />
      </svg>
    </span>
  );
}
