import { invoke, isTauri } from '@tauri-apps/api/core';

export interface StorageFailure {
  code: string;
  message: string;
}
export interface StorageOverview {
  databaseStatus: 'ready';
  databasePath: string;
  schemaVersion: number;
  vaultPath: string;
  vaultStatus: 'ready' | 'unavailable';
  vaultError: StorageFailure | null;
  backupSnapshots: number;
  chatArchives: number;
  vaultBytes: number | null;
}
export interface StorageService {
  getOverview(): Promise<StorageOverview>;
  initializeVault(): Promise<StorageOverview>;
  chooseVaultDirectory(): Promise<StorageOverview | null>;
}

const browserError: StorageFailure = {
  code: 'runtime_unavailable',
  message:
    'Desktop storage is unavailable in browser preview. Open ChatVault Desktop to use the local database and vault.',
};

const safeCodes = new Set([
  'database_unavailable',
  'vault_unavailable',
  'unsafe_path',
  'vault_not_empty',
  'destination_not_empty',
  'foreign_vault',
  'unrecognized_vault',
  'read_only',
  'unsupported_database',
  'migration_failed',
  'scan_limit',
  'storage_busy',
  'storage_unavailable',
  'runtime_unavailable',
]);
export function storageFailure(error: unknown): StorageFailure {
  if (
    error &&
    typeof error === 'object' &&
    'code' in error &&
    'message' in error &&
    typeof error.code === 'string' &&
    safeCodes.has(error.code) &&
    typeof error.message === 'string'
  ) {
    return { code: error.code, message: error.message };
  }
  return {
    code: 'storage_unavailable',
    message:
      'Local storage could not be reached. Retry the check or restart the desktop application. No storage was reset.',
  };
}

async function call<T>(command: string): Promise<T> {
  if (!isTauri()) throw browserError;
  try {
    return await invoke<T>(command);
  } catch (error) {
    throw storageFailure(error);
  }
}

/** The only storage IPC boundary. No frontend path strings or SQL are accepted. */
export const nativeStorageService: StorageService = {
  getOverview: () => call<StorageOverview>('get_storage_overview'),
  initializeVault: () => call<StorageOverview>('initialize_vault'),
  chooseVaultDirectory: () =>
    call<StorageOverview | null>('choose_vault_directory'),
};

export function formatBytes(bytes: number | null): string {
  if (bytes === null) return 'Unavailable';
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB', 'PB'];
  let size = bytes / 1024;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return `${new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(size)} ${units[unit]}`;
}
