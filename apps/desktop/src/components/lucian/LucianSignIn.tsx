/* Lucian UX/UI Prompt #403 · MIT License (c) 2026 Seokwoooo · https://lucianlabs.dev/uxui-prompt/#403 */
// Entrada com rótulos flutuantes, olho que abre e fecha, campo que treme no erro
// e botão que encolhe em círculo enquanto confere e vira check quando entra.
import { useEffect, useId, useRef, useState } from 'react';

type Phase = 'idle' | 'busy' | 'ok';

function FloatField({
  id,
  label,
  value,
  onChange,
  type = 'text',
  autoComplete,
  inputMode,
  error,
  shake,
  describedBy,
  eye,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  autoComplete?: string;
  inputMode?: 'numeric' | 'text';
  error?: boolean;
  shake?: number;
  describedBy?: string;
  eye?: boolean;
}) {
  const [focus, setFocus] = useState(false);
  const [show, setShow] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (!el || !shake) return;
    el.classList.remove('is-shake');
    void el.offsetWidth;
    el.classList.add('is-shake');
  }, [shake]);
  const cls = ['u403-f', eye ? 'u403-pw' : '', focus ? 'is-f' : '', focus || value ? 'is-up' : '', error ? 'is-err' : '']
    .filter(Boolean)
    .join(' ');
  return (
    <div ref={ref} className={cls}>
      <input
        id={id}
        className="u403-in"
        type={eye ? (show ? 'text' : 'password') : type}
        autoComplete={autoComplete}
        inputMode={inputMode}
        spellCheck={false}
        value={value}
        aria-invalid={error || undefined}
        aria-describedby={error ? describedBy : undefined}
        onFocus={() => setFocus(true)}
        onBlur={() => setFocus(false)}
        onChange={e => onChange(e.target.value)}
      />
      <label htmlFor={id}>{label}</label>
      {eye && (
        <button
          type="button"
          className="u403-eye"
          aria-label={show ? 'Ocultar senha' : 'Mostrar senha'}
          aria-pressed={show}
          onClick={() => setShow(s => !s)}
        >
          <svg viewBox="0 0 20 20" aria-hidden="true">
            <g className="lid">
              <path d="M2 10s3-5.2 8-5.2S18 10 18 10s-3 5.2-8 5.2S2 10 2 10z" />
              <circle cx="10" cy="10" r="2.4" />
            </g>
            <g className="lash">
              <path d="M3 10.4c2 2.4 4.4 3.4 7 3.4s5-1 7-3.4M6 13l-1 1.6M10 14v1.8M14 13l1 1.6" />
            </g>
          </svg>
        </button>
      )}
    </div>
  );
}

export function LucianSignIn({
  onSubmit,
  error,
}: {
  onSubmit: (username: string, password: string, code: string) => Promise<boolean>;
  error: string | null;
}) {
  const uid = useId();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [code, setCode] = useState('');
  const [phase, setPhase] = useState<Phase>('idle');
  const [shake, setShake] = useState(0);
  const [failed, setFailed] = useState(false);
  const msgId = uid + 'm';
  const ready = Boolean(username.trim() && password);

  return (
    <div className="ux u403">
      <form
        className="u403-form"
        noValidate
        onSubmit={e => {
          e.preventDefault();
          if (phase !== 'idle') return;
          if (!ready) {
            setFailed(true);
            setShake(s => s + 1);
            return;
          }
          setFailed(false);
          setPhase('busy');
          void onSubmit(username.trim(), password, code.trim()).then(ok => {
            setPhase(ok ? 'ok' : 'idle');
            if (!ok) {
              setFailed(true);
              setShake(s => s + 1);
            }
          });
        }}
      >
        <FloatField id={uid + 'u'} label="Usuário ou e-mail do Atrium" value={username} autoComplete="username"
          onChange={v => { setUsername(v); setFailed(false); }} error={failed && !username.trim()} shake={!username.trim() ? shake : 0} describedBy={msgId} />
        <FloatField id={uid + 'p'} label="Senha do Atrium" value={password} autoComplete="current-password" eye
          onChange={v => { setPassword(v); setFailed(false); }} error={failed && Boolean(username.trim())} shake={username.trim() ? shake : 0} describedBy={msgId} />
        <FloatField id={uid + 'c'} label="Código 2FA (se ativo)" value={code} autoComplete="one-time-code" inputMode="numeric"
          onChange={setCode} />
        <p id={msgId} className={'u403-msg' + (failed ? ' is-on' : '')} aria-live="polite">
          {failed ? (ready ? error ?? 'Usuário ou senha não conferem.' : 'Informe usuário e senha.') : ''}
        </p>
        <button
          type="submit"
          className={'u403-go' + (phase === 'busy' ? ' is-busy' : phase === 'ok' ? ' is-ok' : '')}
          aria-busy={phase === 'busy' || undefined}
        >
          <span>Entrar com conta Atrium</span>
          <i aria-hidden="true" />
          <svg viewBox="0 0 22 22" aria-hidden="true"><path d="M5.5 11.5l4 4L17 7.5" /></svg>
        </button>
      </form>
      <p className="u403-live" role="status">
        {phase === 'busy' ? 'Conferindo' : phase === 'ok' ? 'Entrada liberada' : ''}
      </p>
    </div>
  );
}
