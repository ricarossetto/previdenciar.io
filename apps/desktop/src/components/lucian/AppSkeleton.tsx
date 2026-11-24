/* Lucian UX/UI Prompt #439 · MIT License (c) 2026 Seokwoooo · https://lucianlabs.dev/uxui-prompt/#439 */
// Esqueleto com brilho que corre, no lugar e no tamanho do conteúdo que vai chegar (menu, cabeçalho e cartões).
export function AppSkeleton({ message }: { message: string }) {
  return (
    <div className="ux u439 u439-app is-loading" role="status" aria-live="polite" data-od-id="initial-loading">
      <span className="u439-msg">{message}</span>
      <div className="u439-sk" aria-hidden="true">
        <aside className="u439-side">
          <i className="u439-logo" />
          {[0, 1, 2, 3, 4, 5].map(n => <i key={n} className="u439-nav" style={{ ['--d' as string]: n * 0.04 + 's' }} />)}
        </aside>
        <main className="u439-main">
          <i className="u439-head" />
          <div className="u439-grid">
            {[0, 1, 2].map(n => (
              <div key={n} className="u439-c" style={{ ['--d' as string]: 0.12 + n * 0.08 + 's' }}>
                <i className="a" />
                <i className="b" />
                <i className="c" />
              </div>
            ))}
          </div>
          <i className="u439-block" />
        </main>
      </div>
    </div>
  );
}
