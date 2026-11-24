// Verificação de saúde da API publicada.
//
// Uso: node scripts/check-health.mjs [url]
// Padrão: https://previdenciario.atrium.adv.br/api/health
//
// Falha com código 1 quando a resposta não for JSON com status "ok".
// Não usa credenciais e não imprime nada sensível.
//
// A regra de aprovação vem de scripts/health-core.mjs, o mesmo módulo usado
// pelos testes da interface. Aqui ficam apenas a rede e a saída de console.

import { readHealth } from './health-core.mjs';

const URL_PADRAO = 'https://previdenciario.atrium.adv.br/api/health';

async function main() {
  const alvo = process.argv[2] || URL_PADRAO;

  let resposta;
  try {
    resposta = await fetch(alvo, { headers: { accept: 'application/json' } });
  } catch (erro) {
    console.error(`FALHA: não foi possível consultar ${alvo}: ${erro.message}`);
    process.exitCode = 1;
    return;
  }

  let corpo;
  try {
    corpo = await resposta.text();
  } catch (erro) {
    console.error(`FALHA: não foi possível ler a resposta de ${alvo}: ${erro.message}`);
    process.exitCode = 1;
    return;
  }

  const tipo = resposta.headers.get('content-type') || '';
  const resultado = readHealth(resposta.status, tipo, corpo);

  if (resultado.ok) {
    console.log(`OK: ${resultado.detail} (${alvo})`);
    return;
  }

  console.error(`FALHA: ${alvo} — ${resultado.detail}`);
  process.exitCode = 1;
}

await main();
