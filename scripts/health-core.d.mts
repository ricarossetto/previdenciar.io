// Declarações de tipo do núcleo único em scripts/health-core.mjs.
// Mantidas ao lado do arquivo para que o aplicativo (TypeScript) use a mesma
// implementação do script de linha de comando.

export type HealthReason =
  | 'ok'
  | 'http-error'
  | 'empty'
  | 'not-json'
  | 'missing-status'
  | 'status-not-ok';

export type HealthResult = {
  ok: boolean;
  reason: HealthReason;
  detail: string;
};

export function pareceJson(contentType: string | null | undefined, body: string | null | undefined): boolean;

export function readHealth(
  status: number,
  contentType: string | null | undefined,
  body: string | null | undefined,
): HealthResult;
