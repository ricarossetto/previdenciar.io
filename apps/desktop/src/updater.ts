// Verificação e instalação de atualizações via tauri-plugin-updater.
// O endpoint e a chave pública vêm de tauri.conf.json. Nunca bloqueia o app:
// qualquer falha vira mensagem, não exceção não tratada.
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export type UpdateResult =
  | { kind: 'current'; message: string }
  | { kind: 'installed'; message: string }
  | { kind: 'error'; message: string };

export async function checkForUpdates(): Promise<UpdateResult> {
  try {
    const update = await check();
    if (!update) {
      return { kind: 'current', message: 'Você está na versão mais recente.' };
    }
    await update.downloadAndInstall();
    await relaunch();
    return { kind: 'installed', message: 'Atualização instalada. Reiniciando...' };
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return { kind: 'error', message: `Não foi possível verificar atualizações: ${message}` };
  }
}
