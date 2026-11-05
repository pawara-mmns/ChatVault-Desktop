import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { expect, it, vi } from 'vitest';
import { NativeInspector } from '../features/backups/NativeInspector';
import type {
  NativeInspection,
  NativePreview,
  NativeService,
} from '../services/native';
import { MessageBubble } from '../features/viewer/MessageTimeline';
import { messageTime, messageDay, type SavedMessage } from '../services/viewer';
const inspection: NativeInspection = {
  snapshotId: 'snapshot-1',
  sourceApp: 'business',
  createdAt: 1000,
  phase: 'completed_with_warnings',
  inventoryComplete: false,
  warnings: ['timeout'],
  capability: 'INCOMPLETE_BACKUP',
  candidates: [
    {
      fileId: 'file-1',
      filename: 'msgstore.db.crypt15',
      bytes: 10000,
      format: 'crypt15',
      capability: 'SUPPORTED_AND_KEY_REQUIRED',
      keyRequirement: 'matching hexadecimal root key',
      hashVerified: true,
      duplicateOf: null,
      issue: null,
    },
  ],
  media: [['images', 3, 100]],
  missingFiles: 0,
  totalFiles: 4,
  existingArchive: null,
};
const preview: NativePreview = {
  token: 'preview-1',
  snapshotId: 'snapshot-1',
  fileId: 'file-1',
  format: 'crypt15',
  schema: 'message-chat-jid-v1',
  parserVersion: 1,
  counts: {
    conversations: 3,
    messages: 7,
    mediaMatched: 1,
    mediaMissing: 1,
    unsupported: 1,
  },
  warnings: [],
  archiveId: null,
};
function service(): NativeService {
  return {
    inspect: vi.fn().mockResolvedValue(inspection),
    verify: vi.fn().mockResolvedValue(preview),
    keyFile: vi.fn().mockResolvedValue(preview),
    import: vi.fn().mockResolvedValue({ ...preview, archiveId: 'archive-1' }),
    progress: vi.fn().mockResolvedValue({
      phase: 'inspecting',
      bytes: 0,
      messages: 0,
      cancelRequested: false,
      error: null,
    }),
    cancel: vi.fn().mockResolvedValue(undefined),
  };
}
function page(s: NativeService) {
  return render(
    <MemoryRouter>
      <NativeInspector snapshotId="snapshot-1" service={s} />
    </MemoryRouter>,
  );
}
async function inspect() {
  await userEvent.click(screen.getByRole('button', { name: 'Inspect Backup' }));
  await screen.findByText('msgstore.db.crypt15', { exact: false });
}
async function verify() {
  await userEvent.click(
    screen.getByRole('checkbox', {
      name: /I consent to local plaintext processing/,
    }),
  );
  await userEvent.type(
    screen.getByLabelText('Matching crypt15 root key'),
    '07'.repeat(32),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Verify compatibility' }),
  );
}
it('inspection is explicit and encrypted filenames do not enable reading', async () => {
  const s = service();
  page(s);
  expect(s.inspect).not.toHaveBeenCalled();
  await inspect();
  expect(s.inspect).toHaveBeenCalledWith('snapshot-1');
  expect(screen.getByText(/INCOMPLETE BACKUP/)).toBeInTheDocument();
  expect(
    screen.queryByRole('link', { name: /Read Chats/ }),
  ).not.toBeInTheDocument();
  expect(
    screen.getByRole('button', { name: 'Verify compatibility' }),
  ).toBeDisabled();
  expect(
    screen.getByText(
      /messages cannot currently be opened without a matching key/,
    ),
  ).toBeInTheDocument();
});
it('requires independent processing and persistence consent before viewer integration', async () => {
  const s = service();
  page(s);
  await inspect();
  await verify();
  await screen.findByText(/authenticated input/);
  expect(s.verify).toHaveBeenCalledWith(
    'snapshot-1',
    'file-1',
    '07'.repeat(32),
    true,
  );
  expect(
    screen.getByRole('button', { name: 'Import readable chats' }),
  ).toBeDisabled();
  expect(s.import).not.toHaveBeenCalled();
  expect(
    screen.getByText(/search index and vault are unencrypted/),
  ).toBeInTheDocument();
  await userEvent.click(
    screen.getByRole('checkbox', {
      name: /I explicitly consent to persisting/,
    }),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Import readable chats' }),
  );
  expect(
    await screen.findByRole('link', { name: 'Read Chats' }),
  ).toHaveAttribute('href', '/chats?archive=archive-1');
  expect(s.import).toHaveBeenCalledWith('preview-1', true);
});
it('wrong key feedback clears the field and never exposes the submitted value', async () => {
  const s = service();
  vi.mocked(s.verify).mockRejectedValue({
    code: 'native_auth',
    message: 'Authentication failed. No readable archive was saved.',
  });
  page(s);
  await inspect();
  await verify();
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Authentication failed',
  );
  expect(screen.getByLabelText('Matching crypt15 root key')).toHaveValue('');
  expect(
    screen.queryByRole('link', { name: 'Read Chats' }),
  ).not.toBeInTheDocument();
  expect(s.import).not.toHaveBeenCalled();
});
it('crypt14 opens only a native key-file picker after consent', async () => {
  const s = service();
  vi.mocked(s.inspect).mockResolvedValue({
    ...inspection,
    candidates: [
      {
        ...inspection.candidates[0]!,
        format: 'crypt14',
        filename: 'msgstore.db.crypt14',
        keyRequirement: 'matching local key file',
      },
    ],
  });
  page(s);
  await userEvent.click(screen.getByRole('button', { name: 'Inspect Backup' }));
  const action = await screen.findByRole('button', {
    name: 'Select matching key file & verify compatibility',
  });
  expect(action).toBeDisabled();
  expect(
    screen.queryByLabelText('Matching crypt15 root key'),
  ).not.toBeInTheDocument();
  await userEvent.click(
    screen.getByRole('checkbox', {
      name: /I consent to local plaintext processing/,
    }),
  );
  await userEvent.click(action);
  await waitFor(() =>
    expect(s.keyFile).toHaveBeenCalledWith('snapshot-1', 'file-1', true),
  );
  expect(s.verify).not.toHaveBeenCalled();
});
it('unsupported header states offer the existing export importer without unlock', async () => {
  const s = service();
  vi.mocked(s.inspect).mockResolvedValue({
    ...inspection,
    capability: 'UNSUPPORTED_FORMAT',
    candidates: [
      {
        ...inspection.candidates[0]!,
        capability: 'UNSUPPORTED_FORMAT',
        format: 'unrecognized (crypt20)',
      },
    ],
  });
  page(s);
  await inspect();
  expect(
    screen.queryByRole('button', { name: 'Verify compatibility' }),
  ).not.toBeInTheDocument();
  expect(screen.getByRole('link', { name: /manual TXT\/ZIP/ })).toHaveAttribute(
    'href',
    '/import',
  );
});
it('inspection errors do not create a readable-data screen', async () => {
  const s = service();
  vi.mocked(s.inspect).mockRejectedValue({
    code: 'native_integrity',
    message: 'Snapshot manifest is unavailable.',
  });
  page(s);
  await userEvent.click(screen.getByRole('button', { name: 'Inspect Backup' }));
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Snapshot manifest is unavailable',
  );
  expect(screen.queryByText(/Unlock & Read Chats/)).not.toBeInTheDocument();
});
it('import rollback returns to the key workflow without a fake viewer link', async () => {
  const s = service();
  vi.mocked(s.import).mockRejectedValue({
    code: 'native_database',
    message: 'Import rolled back.',
  });
  page(s);
  await inspect();
  await verify();
  await userEvent.click(
    await screen.findByRole('checkbox', {
      name: /I explicitly consent to persisting/,
    }),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Import readable chats' }),
  );
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Import rolled back',
  );
  expect(
    screen.queryByRole('link', { name: 'Read Chats' }),
  ).not.toBeInTheDocument();
});
it('existing committed archives open without a new key or duplicate import', async () => {
  const s = service();
  vi.mocked(s.inspect).mockResolvedValue({
    ...inspection,
    existingArchive: 'stored-archive',
  });
  page(s);
  await inspect();
  expect(
    screen.getByRole('link', { name: /existing native archive/ }),
  ).toHaveAttribute('href', '/chats?archive=stored-archive');
  expect(
    screen.queryByRole('button', { name: 'Verify compatibility' }),
  ).not.toBeInTheDocument();
});
it('native messages preserve direction, UTC timestamps and reliable reply navigation', () => {
  const message: SavedMessage = {
    id: 'm2',
    sequence: 2,
    sender: 'You',
    text: 'Synthetic outgoing',
    originalTimestamp: '1700000002000',
    localTimestamp: null,
    classification: 'message',
    attachments: [],
    nativeDirection: 'outgoing',
    nativeTimestampMs: 1700000002000,
    nativeReplyId: 'm1',
  };
  render(
    <MemoryRouter>
      <MessageBubble
        message={message}
        query=""
        onEnlarge={vi.fn()}
        conversationId="c1"
      />
    </MemoryRouter>,
  );
  expect(screen.getByRole('article')).toHaveClass('native-outgoing');
  expect(
    screen.getByRole('link', { name: 'View replied-to message' }),
  ).toHaveAttribute('href', '/chats?conversation=c1&message=m1');
  expect(messageTime(message)).toContain('(UTC)');
  expect(messageDay(message)).toContain('2023-11-14');
});
