import type {
  ArchiveSummary,
  ImportPreview,
  SelectedImport,
} from '../services/imports';
export const selectedFixture: SelectedImport = {
  token: 'synthetic-session',
  filename: 'Synthetic export.txt',
  sizeBytes: 256,
  sourceFormat: 'txt',
  duplicateArchive: null,
};
export const previewFixture: ImportPreview = {
  selection: selectedFixture,
  title: 'Synthetic conversation',
  messageCount: 2,
  participantCount: 2,
  participants: ['Fixture Alice', 'Fixture Bob'],
  mediaCount: 1,
  attachmentCount: 2,
  missingAttachmentCount: 1,
  dateOrderRequired: false,
  dateOrder: 'auto',
  encoding: 'UTF-8',
  samples: [
    {
      sender: 'Fixture Alice',
      text: 'Synthetic hello\nSecond fixture line 👋',
      originalTimestamp: '31/12/2026, 09:30',
      localTimestamp: '2026-12-31T09:30:00',
      classification: 'message',
    },
  ],
  diagnostics: [
    {
      code: 'timezone_unknown',
      severity: 'warning',
      description: 'Export times have no known timezone.',
      occurrenceCount: 1,
      lineNumber: null,
    },
  ],
};
export const archiveFixture: ArchiveSummary = {
  id: 'synthetic-archive',
  title: 'Synthetic conversation',
  originalFilename: 'Synthetic export.txt',
  sourceFormat: 'txt',
  importedAt: 1796340000000,
  messageCount: 2,
  mediaCount: 1,
  missingMediaCount: 1,
  totalBytes: 512,
  importStatus: 'complete',
  originalSha256: 'a'.repeat(64),
  dateOrder: 'auto',
  textEncoding: 'UTF-8',
  diagnostics: previewFixture.diagnostics,
};
