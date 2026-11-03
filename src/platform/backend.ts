import { invoke, isTauri } from '@tauri-apps/api/core';
export interface BackendHealth {
  status: 'ok';
  version: string;
  platform: string;
}
export async function checkBackend(): Promise<BackendHealth> {
  if (!isTauri())
    throw new Error(
      'Browser preview: open the desktop app to check the Rust backend.',
    );
  const result = await invoke<BackendHealth>('health_check');
  if (result.status !== 'ok')
    throw new Error('The desktop backend returned an unexpected response.');
  return result;
}
