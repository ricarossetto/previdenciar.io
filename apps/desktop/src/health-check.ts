// Conferência de saúde da API que não se contenta com HTTP 200.
//
// A implementação fica em scripts/health-core.mjs para ser a mesma usada pelo
// script de linha de comando (scripts/check-health.mjs). Este arquivo só
// reexporta, com os tipos, para o aplicativo.

export { pareceJson, readHealth } from '../../../scripts/health-core.mjs';
export type { HealthReason, HealthResult } from '../../../scripts/health-core.mjs';
