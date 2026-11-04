import type {
  Conversation,
  SavedMessage,
  Attachment,
  MediaAccess,
} from '../services/viewer';
export const conversationFixture: Conversation = {
  id: 'synthetic-conversation',
  archiveId: 'synthetic-archive',
  title: 'Synthetic conversation',
  originalFilename: 'Synthetic.txt',
  importedAt: 1793746800000,
  messageCount: 12000,
  attachmentCount: 1,
  mediaCount: 1,
  lastPreview: 'A real fixture excerpt',
  lastLocalTimestamp: '2026-05-04T09:04:00',
  lastOriginalTimestamp: '04/05/2026, 09:04',
  participants: ['Fixture Alice', 'Fixture Bob'],
  dateOrder: 'day-first',
};
export function messageFixture(
  sequence: number,
  overrides: Partial<SavedMessage> = {},
): SavedMessage {
  return {
    id: `synthetic-message-${sequence}`,
    sequence,
    sender: 'Fixture Alice',
    text: `Synthetic record ${sequence}`,
    originalTimestamp: '04/05/2026, 09:01',
    localTimestamp: '2026-05-04T09:01:00',
    classification: 'message',
    attachments: [],
    ...overrides,
  };
}
export const imageFixture: Attachment = {
  id: 'synthetic-attachment',
  mediaId: 'synthetic-media',
  filename: 'Synthetic.png',
  mediaType: 'image/png',
  sizeBytes: 68,
  missing: false,
};
export const mediaAccessFixture: MediaAccess = {
  token: 'synthetic-private-token',
  metadata: {
    id: 'synthetic-media',
    filename: 'Synthetic.png',
    sizeBytes: 68,
    mime: 'image/png',
    kind: 'image',
    width: 1,
    height: 1,
  },
};
