import { invoke, isTauri } from '@tauri-apps/api/core';
import type { StorageFailure } from './storage';
export type DateOrder = 'auto' | 'day-first' | 'month-first';
export interface ImportDiagnostic {
  code: string;
  severity: 'warning' | 'info';
  description: string;
  occurrenceCount: number;
  lineNumber: number | null;
}
export interface SelectedImport {
  token: string;
  filename: string;
  sizeBytes: number;
  sourceFormat: 'txt' | 'zip';
  duplicateArchive: string | null;
}
export interface MessageSample {
  sender: string | null;
  text: string;
  originalTimestamp: string | null;
  localTimestamp: string | null;
  classification: string;
}
export interface ImportPreview {
  selection: SelectedImport;
  title: string;
  messageCount: number;
  participantCount: number;
  participants: string[];
  mediaCount: number;
  attachmentCount: number;
  missingAttachmentCount: number;
  dateOrderRequired: boolean;
  dateOrder: DateOrder;
  encoding: string;
  samples: MessageSample[];
  diagnostics: ImportDiagnostic[];
}
export interface ArchiveSummary {
  id: string;
  title: string;
  originalFilename: string | null;
  sourceFormat: string | null;
  importedAt: number;
  messageCount: number;
  mediaCount: number;
  missingMediaCount: number;
  totalBytes: number;
  importStatus: 'legacy' | 'complete';
  originalSha256: string | null;
  dateOrder: string | null;
  textEncoding: string | null;
  diagnostics: ImportDiagnostic[];
}
export interface ArchivePage {
  items: ArchiveSummary[];
  total: number;
  offset: number;
  limit: number;
}
export function desktopImportAvailable() {
  return isTauri();
}
const codes = new Set([
  'invalid_file',
  'invalid_encoding',
  'unsupported_format',
  'source_changed',
  'import_limit',
  'invalid_zip',
  'unsupported_zip',
  'unsafe_zip',
  'unsafe_import',
  'import_io',
  'import_database',
  'import_recovery',
  'import_busy',
  'import_expired',
  'import_not_analyzed',
  'duplicate_import',
  'ambiguous_dates',
  'invalid_title',
  'invalid_filter',
  'archive_missing',
  'vault_changed',
  'import_failed',
  'vault_unavailable',
  'unsafe_path',
  'read_only',
  'database_unavailable',
  'runtime_unavailable',
]);
export function importFailure(error: unknown): StorageFailure {
  if (
    error &&
    typeof error === 'object' &&
    'code' in error &&
    'message' in error &&
    typeof error.code === 'string' &&
    codes.has(error.code) &&
    typeof error.message === 'string'
  )
    return { code: error.code, message: error.message };
  return {
    code: 'import_failed',
    message:
      'The local import request could not finish. Check Archives before choosing the file again. No original source file was changed.',
  };
}
async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    throw {
      code: 'runtime_unavailable',
      message:
        'Chat importing and the archive library require the desktop runtime. Open ChatVault Desktop to continue.',
    };
  try {
    return args ? await invoke<T>(command, args) : await invoke<T>(command);
  } catch (error) {
    throw importFailure(error);
  }
}
export interface ImportService {
  select(): Promise<SelectedImport | null>;
  analyze(token: string, dateOrder: DateOrder): Promise<ImportPreview>;
  confirm(token: string, title: string): Promise<ArchiveSummary>;
  cancel(token: string): Promise<void>;
}
export interface ArchiveService {
  list(query?: string, offset?: number, limit?: number): Promise<ArchivePage>;
  summary(id: string): Promise<ArchiveSummary>;
}
export const importService: ImportService = {
  select: () => call<SelectedImport | null>('select_import_file'),
  analyze: (token: string, dateOrder: DateOrder) =>
    call<ImportPreview>('analyze_import', { token, dateOrder }),
  confirm: (token: string, title: string) =>
    call<ArchiveSummary>('confirm_import', { token, title }),
  cancel: (token: string) => call<void>('cancel_import', { token }),
};
export const archiveService: ArchiveService = {
  list: (query = '', offset = 0, limit = 20) =>
    call<ArchivePage>('list_archives', { query, offset, limit }),
  summary: (id: string) => call<ArchiveSummary>('get_archive_summary', { id }),
};
