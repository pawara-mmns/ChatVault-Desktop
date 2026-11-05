import { invoke, isTauri } from '@tauri-apps/api/core';
export interface NativeCandidate {
  fileId: string;
  filename: string;
  bytes: number;
  format: string;
  capability: string;
  keyRequirement: string;
  hashVerified: boolean;
  duplicateOf: string | null;
  issue: string | null;
}
export interface NativeInspection {
  localIntegrity?: string;
  snapshotId: string;
  sourceApp: string | null;
  createdAt: number;
  phase: string;
  inventoryComplete: boolean;
  warnings: string[];
  capability: string;
  candidates: NativeCandidate[];
  media: [string, number, number][];
  missingFiles: number;
  totalFiles: number;
  existingArchive: string | null;
}
export interface NativeCounts {
  conversations: number;
  messages: number;
  mediaMatched: number;
  mediaMissing: number;
  unsupported: number;
}
export interface NativePreview {
  token: string;
  snapshotId: string;
  fileId: string;
  format: string;
  schema: string;
  parserVersion: number;
  counts: NativeCounts;
  warnings: string[];
  archiveId: string | null;
}
export interface NativeProgress {
  phase: string;
  bytes: number;
  messages: number;
  cancelRequested: boolean;
  error: { code: string; message: string } | null;
}
export interface NativeService {
  inspect(id: string): Promise<NativeInspection>;
  verify(
    id: string,
    fileId: string,
    key: string,
    consent: boolean,
  ): Promise<NativePreview>;
  keyFile(
    id: string,
    fileId: string,
    consent: boolean,
  ): Promise<NativePreview | null>;
  import(token: string, consent: boolean): Promise<NativePreview>;
  progress(): Promise<NativeProgress>;
  cancel(): Promise<void>;
}
async function call<T>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    throw {
      code: 'runtime_unavailable',
      message: 'Open the Windows desktop app to inspect a saved snapshot.',
    };
  try {
    return await invoke<T>(name, args);
  } catch (e) {
    throw nativeError(e);
  }
}
export function nativeError(e: unknown) {
  if (
    e &&
    typeof e === 'object' &&
    'code' in e &&
    'message' in e &&
    typeof e.code === 'string' &&
    typeof e.message === 'string' &&
    /^[a-z_]{1,50}$/.test(e.code)
  )
    return { code: e.code, message: e.message };
  return {
    code: 'native_unavailable',
    message:
      'The local native reader could not finish. Original backups were preserved.',
  };
}
export const nativeService: NativeService = {
  inspect: (snapshotId) => call('inspect_native_backup', { snapshotId }),
  verify: (snapshotId, fileId, key, processConsent) =>
    call('verify_native_key', { snapshotId, fileId, key, processConsent }),
  keyFile: (snapshotId, fileId, processConsent) =>
    call('verify_native_key_file', { snapshotId, fileId, processConsent }),
  import: (token, plaintextConsent) =>
    call('import_native_backup', { token, plaintextConsent }),
  progress: () => call('get_native_reader_progress'),
  cancel: () => call('cancel_native_reader'),
};
