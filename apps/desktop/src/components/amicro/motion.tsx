import { createContext, useContext, useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { MotionConfig, motion, useMotionValue, useSpring, useTransform } from 'motion/react';

// Adapted from Amicro FadeUp, TiltCard and spring presets (MIT; see LICENSE).
export const springs = {
  snappy: { type: 'spring', stiffness: 400, damping: 28, mass: 0.8 },
  smooth: { type: 'spring', stiffness: 220, damping: 24, mass: 1 },
} as const;
const preferenceKey = 'previus.amicro.reduce-motion';
const MotionPolicy = createContext({ enabled: false, reduced: true, toggle: () => {} });

export function MotionProvider({ children }: { children: ReactNode }) {
  const [manual, setManual] = useState(() => {
    try { return localStorage.getItem(preferenceKey) === 'true'; } catch { return false; }
  });
  const [system, setSystem] = useState(() =>
    typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  useEffect(() => {
    if (typeof window.matchMedia !== 'function') return;
    const media = window.matchMedia('(prefers-reduced-motion: reduce)');
    const update = () => setSystem(media.matches);
    update();
    media.addEventListener('change', update);
    return () => media.removeEventListener('change', update);
  }, []);
  const reduced = system || manual;
  useEffect(() => {
    document.documentElement.dataset.amicroMotion = reduced ? 'off' : 'on';
    return () => { delete document.documentElement.dataset.amicroMotion; };
  }, [reduced]);
  const toggle = () => {
    const next = !manual;
    setManual(next);
    try { localStorage.setItem(preferenceKey, String(next)); } catch { /* Optional local preference. */ }
  };
  return <MotionPolicy.Provider value={{ enabled: !reduced, reduced, toggle }}>
    <MotionConfig reducedMotion={reduced ? 'always' : 'never'}>{children}</MotionConfig>
  </MotionPolicy.Provider>;
}
export const useMotionPolicy = () => useContext(MotionPolicy);

export function PageTransition({ children, className, pointer, page }: {
  children: ReactNode; className: string; pointer: boolean; page: string;
}) {
  const { enabled } = useMotionPolicy();
  return <motion.div className={className} data-od-id={'page-' + page} data-motion={pointer ? 'pointer' : 'none'}
    initial={enabled && pointer ? { opacity: 0.65, y: 12 } : false}
    animate={{ opacity: 1, y: 0 }} transition={enabled ? springs.smooth : { duration: 0 }}>{children}</motion.div>;
}

export function NavIndicator({ id }: { id: string }) {
  const { enabled } = useMotionPolicy();
  // A moving decorative layer is unnecessary in reduced-motion mode.
  if (!enabled) return null;
  return <motion.span aria-hidden="true" className="amicro-nav-indicator" layoutId={enabled ? id : undefined}
    transition={enabled ? springs.snappy : { duration: 0 }} />;
}

export function FadeUp({ children, className, delay = 0 }: {
  children: ReactNode; className?: string; delay?: number;
}) {
  const { enabled } = useMotionPolicy();
  return <motion.div className={className} initial={enabled ? { opacity: 0.7, y: 14 } : false}
    animate={{ opacity: 1, y: 0 }} transition={enabled ? { ...springs.smooth, delay } : { duration: 0 }}>
    {children}
  </motion.div>;
}

export function TiltCard({ children, index }: { children: ReactNode; index: number }) {
  const { enabled } = useMotionPolicy();
  const ref = useRef<HTMLDivElement>(null);
  const x = useMotionValue(0), y = useMotionValue(0);
  const rotateX = useSpring(useTransform(y, [-0.5, 0.5], [2, -2]), springs.smooth);
  const rotateY = useSpring(useTransform(x, [-0.5, 0.5], [-2, 2]), springs.smooth);
  const reset = () => { x.set(0); y.set(0); };
  useEffect(() => { if (!enabled) { x.set(0); y.set(0); } }, [enabled, x, y]);
  return <motion.div ref={ref} className="amicro-action-card" data-amicro="tilt-card"
    initial={enabled ? { opacity: 0.7, y: 14 } : false} animate={{ opacity: 1, y: 0 }}
    transition={enabled ? { ...springs.smooth, delay: index * 0.045 } : { duration: 0 }}
    style={enabled ? { rotateX, rotateY } : { transform: 'none' }} onPointerLeave={reset}
    onPointerMove={event => {
      if (!enabled || event.pointerType !== 'mouse' || !ref.current) return;
      const box = ref.current.getBoundingClientRect();
      x.set((event.clientX - box.left) / box.width - 0.5);
      y.set((event.clientY - box.top) / box.height - 0.5);
    }}>{children}</motion.div>;
}

export function MotionLoader() {
  return <span className="amicro-loader" aria-hidden="true"><i /><i /><i /></span>;
}
