import { vi } from 'vitest';
import type { StorageOverview, StorageService } from '../services/storage';

/** Isolated test fixture only; the production frontend never fabricates database results. */
export const freshStorage: StorageOverview = {
  databaseStatus: 'ready',
  databasePath: 'C:\\TestOnly\\database\\chatvault.sqlite3',
  schemaVersion: 1,
  vaultPath: 'C:\\TestOnly\\ChatVault\\vault',
  vaultStatus: 'ready',
  vaultError: null,
  backupSnapshots: 0,
  chatArchives: 0,
  vaultBytes: 0,
};
export function storageFixture(): StorageService {
  return {
    getOverview: vi.fn().mockResolvedValue({ ...freshStorage }),
    initializeVault: vi.fn().mockResolvedValue({ ...freshStorage }),
    chooseVaultDirectory: vi.fn().mockResolvedValue(null),
  };
}
