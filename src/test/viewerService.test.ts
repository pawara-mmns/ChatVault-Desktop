import { beforeEach, it, expect, vi } from 'vitest';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { viewerService } from '../services/viewer';
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
  convertFileSrc: vi.fn(),
}));
beforeEach(() => {
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(invoke).mockResolvedValue({});
});
it('rejects browser reads without invoking native commands', async () => {
  vi.mocked(isTauri).mockReturnValue(false);
  await expect(viewerService.conversation('id')).rejects.toMatchObject({
    code: 'runtime_unavailable',
  });
  expect(invoke).not.toHaveBeenCalled();
});
it('requests message pages and media by identifiers without arbitrary filesystem paths', async () => {
  await viewerService.messages('conversation', 'message', 'older');
  expect(invoke).toHaveBeenCalledWith('read_messages', {
    conversationId: 'conversation',
    cursor: 'message',
    direction: 'older',
    limit: 50,
  });
  await viewerService.authorize('media-id');
  expect(invoke).toHaveBeenLastCalledWith('authorize_media', {
    mediaId: 'media-id',
  });
});
it('does not expose arbitrary backend diagnostic strings', async () => {
  vi.mocked(invoke).mockRejectedValue(new Error('private OS data'));
  await expect(viewerService.search('word')).rejects.toMatchObject({
    code: 'viewer_failed',
  });
});
