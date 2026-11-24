/* Lucian UX/UI Prompt #426 · MIT License (c) 2026 Seokwoooo · https://lucianlabs.dev/uxui-prompt/#426 */
// Cada dígito rola como odômetro e para no valor. O texto completo fica para leitores de tela.
// Adaptação: altura do dígito em em (1.25em) no lugar de 30px, para servir em tamanhos diferentes.
import { useLayoutEffect, useRef } from 'react';

const DIG = '01234567890123456789'.split('');

export function Odometer({ value }: { value: string }) {
  const ref = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    const root = ref.current;
    if (!root) return;
    const cols = Array.from(root.querySelectorAll<HTMLElement>('.u426-col > span'));
    const rm = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
    cols.forEach(c => {
      c.style.transition = 'none';
      c.style.setProperty('--d', rm ? String(+c.dataset.d! + 10) : '0');
    });
    void root.offsetWidth;
    if (!rm) cols.forEach(c => { c.style.transition = ''; c.style.setProperty('--d', String(+c.dataset.d! + 10)); });
  }, [value]);
  let k = 0;
  return (
    <span className="ux u426-inline">
      <span className="u426-sr">{value}</span>
      <span ref={ref} className="u426-n" aria-hidden="true">
        {value.split('').map((ch, i) =>
          /\d/.test(ch) ? (
            <span key={i} className="u426-col">
              <span data-d={ch} style={{ ['--k' as string]: k++ }}>
                {DIG.map((d, j) => <b key={j}>{d}</b>)}
              </span>
            </span>
          ) : (
            <i key={i}>{ch}</i>
          ),
        )}
      </span>
    </span>
  );
}
