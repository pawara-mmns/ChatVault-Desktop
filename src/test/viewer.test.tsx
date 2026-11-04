import {
  render,
  screen,
  waitFor,
  fireEvent,
  within,
} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, it, expect, vi } from 'vitest';
import { ChatViewerPage } from '../features/viewer/ChatViewerPage';
import {
  MessageTimeline,
  MessageBubble,
  MAX_WINDOW_MESSAGES,
} from '../features/viewer/MessageTimeline';
import { MediaMessage } from '../features/viewer/MediaMessage';
import { MediaLightbox } from '../features/viewer/MediaLightbox';
import { ChatSearch } from '../features/viewer/ChatSearch';
import { Highlight } from '../features/viewer/Highlight';
import { viewerService } from '../services/viewer';
import type * as ViewerModule from '../services/viewer';
import {
  conversationFixture as conversation,
  messageFixture as message,
  imageFixture,
  mediaAccessFixture,
} from './viewerFixture';
vi.mock('../services/viewer', async (original) => {
  const actual = await original<typeof ViewerModule>();
  return {
    ...actual,
    privateMediaUrl: (token: string) =>
      `http://chatvault-media.localhost/${token}`,
    viewerService: {
      conversations: vi.fn(),
      conversation: vi.fn(),
      messages: vi.fn(),
      context: vi.fn(),
      search: vi.fn(),
      media: vi.fn(),
      metadata: vi.fn(),
      authorize: vi.fn(),
      revoke: vi.fn(),
      openDocument: vi.fn(),
    },
  };
});
beforeEach(() => {
  sessionStorage.clear();
  vi.mocked(viewerService.conversations).mockResolvedValue({
    items: [conversation],
    total: 1,
  });
  vi.mocked(viewerService.conversation).mockResolvedValue(conversation);
  vi.mocked(viewerService.messages).mockResolvedValue({
    items: [message(1)],
    hasOlder: false,
    hasNewer: false,
    anchorId: null,
  });
  vi.mocked(viewerService.media).mockResolvedValue({
    items: [imageFixture],
    nextOffset: null,
  });
  vi.mocked(viewerService.authorize).mockResolvedValue(mediaAccessFixture);
  vi.mocked(viewerService.revoke).mockResolvedValue();
  vi.mocked(viewerService.metadata).mockResolvedValue(
    mediaAccessFixture.metadata,
  );
});
function mount(route = '/archives/read?conversation=synthetic-conversation') {
  return render(
    <MemoryRouter initialEntries={[route]}>
      <ChatViewerPage />
    </MemoryRouter>,
  );
}
describe('saved conversation reader', () => {
  it('opens actual stored data, labels senders, preserves Unicode/multiline and dates without guessing me', async () => {
    vi.mocked(viewerService.messages).mockResolvedValue({
      items: [
        message(1, {
          text: 'ආයුබෝවන් 👋\nGrüße\n<img src=x onerror=alert(1)>',
        }),
        message(2, {
          sender: null,
          classification: 'system',
          text: 'Synthetic system record',
        }),
      ],
      hasOlder: false,
      hasNewer: false,
      anchorId: null,
    });
    const { container } = mount();
    expect(await screen.findByText(/ආයුබෝවන්/)).toHaveTextContent('Grüße');
    expect(screen.getByText('System')).toBeInTheDocument();
    expect(screen.getByText('Fixture Alice')).toBeInTheDocument();
    expect(screen.getByText('2026-05-04')).toBeInTheDocument();
    expect(container.querySelector('.message-body')?.textContent).toContain(
      '\n',
    );
    expect(container.querySelector('.message-body img')).toBeNull();
    expect(screen.getByText(/neutral sender alignment/)).toBeInTheDocument();
  });
  it('selects a conversation by keyboard and preserves its selected ID on navigation', async () => {
    const user = userEvent.setup();
    mount('/archives/read');
    const item = await screen.findByRole('button', {
      name: /Synthetic conversation.*real fixture excerpt/,
    });
    item.focus();
    await user.keyboard('{Enter}');
    expect(await screen.findByText('Synthetic record 1')).toBeInTheDocument();
    expect(item).toHaveAttribute('aria-current', 'true');
    expect(sessionStorage.getItem('chatvault:selected-conversation')).toBe(
      conversation.id,
    );
  });
  it('resolves an imported archive to its saved conversation', async () => {
    mount('/archives/read?archive=synthetic-archive');
    await screen.findByText('Synthetic record 1');
    expect(viewerService.conversations).toHaveBeenCalledWith(
      '',
      'synthetic-archive',
      0,
      1,
    );
  });
  it('shows loading, missing conversation errors, and retry', async () => {
    vi.mocked(viewerService.conversation).mockRejectedValueOnce({
      code: 'conversation_missing',
      message: 'Synthetic missing record',
    });
    const user = userEvent.setup();
    mount();
    expect(screen.getByText('Opening saved conversation…')).toBeInTheDocument();
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Synthetic missing record',
    );
    await user.click(
      screen.getByRole('button', { name: 'Retry conversation' }),
    );
    expect(await screen.findByText('Synthetic record 1')).toBeInTheDocument();
  });
  it('shows an empty sidebar and reader when nothing is imported', async () => {
    vi.mocked(viewerService.conversations).mockResolvedValue({
      items: [],
      total: 0,
    });
    mount('/archives/read');
    expect(
      await screen.findByText('Import a chat to start reading.'),
    ).toBeInTheDocument();
    expect(
      screen.getByRole('heading', { name: 'Make room for a conversation.' }),
    ).toBeInTheDocument();
  });
  it('opens and collapses actual participant and media metadata', async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByText('Synthetic record 1');
    await user.click(
      screen.getByRole('button', { name: 'Show conversation information' }),
    );
    expect(await screen.findByText('Synthetic.png')).toBeInTheDocument();
    expect(screen.getByText('Fixture Bob')).toBeInTheDocument();
    await user.click(
      screen.getByRole('button', { name: 'Close conversation information' }),
    );
    expect(
      screen.queryByRole('complementary', { name: 'Conversation information' }),
    ).not.toBeInTheDocument();
  });
  it('bounds repeated timeline pages and uses cursor IDs', async () => {
    vi.mocked(viewerService.messages).mockImplementation(
      async (_id, cursor) => {
        const end = cursor ? Number(cursor.split('-').at(-1)) - 1 : 11999;
        const start = Math.max(0, end - 49);
        return {
          items: Array.from({ length: end - start + 1 }, (_, i) =>
            message(start + i),
          ),
          hasOlder: start > 0,
          hasNewer: end < 11999,
          anchorId: null,
        };
      },
    );
    const user = userEvent.setup();
    const { container } = render(
      <MessageTimeline
        conversationId={conversation.id}
        targetId={null}
        query=""
        onEnlarge={() => {}}
      />,
    );
    await screen.findByText('Synthetic record 11999');
    const started = performance.now();
    for (let i = 0; i < 4; i++) {
      await user.click(
        screen.getByRole('button', { name: 'Load older messages' }),
      );
      await waitFor(() =>
        expect(
          screen.getByRole('button', { name: 'Load older messages' }),
        ).not.toBeDisabled(),
      );
    }
    expect(container.querySelectorAll('.message-record').length).toBe(
      MAX_WINDOW_MESSAGES,
    );
    expect(viewerService.messages).toHaveBeenCalledWith(
      conversation.id,
      'synthetic-message-11950',
      'older',
    );
    expect(
      screen.queryByText('Synthetic record 11999'),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Load newer messages' }),
    ).toBeInTheDocument();
    console.info(
      `PERF jsdom: four older-page transitions ${(performance.now() - started).toFixed(1)}ms; DOM records=${container.querySelectorAll('.message-record').length}`,
    );
  });
  it('loads a search target context and highlights literal text safely', async () => {
    vi.mocked(viewerService.context).mockResolvedValue({
      items: [message(40, { text: 'Find [literal] <script>bad()</script>' })],
      hasOlder: true,
      hasNewer: true,
      anchorId: 'synthetic-message-40',
    });
    const { container } = render(
      <MessageTimeline
        conversationId={conversation.id}
        targetId="synthetic-message-40"
        query="[literal]"
        onEnlarge={() => {}}
      />,
    );
    await screen.findByText('[literal]');
    expect(viewerService.context).toHaveBeenCalledWith(
      conversation.id,
      'synthetic-message-40',
    );
    expect(container.querySelector('mark')).toHaveTextContent('[literal]');
    expect(container.querySelector('script')).toBeNull();
  });
  it('searches globally and jumps to the correct saved message with previous/next navigation', async () => {
    const hit = {
      messageId: 'synthetic-message-40',
      conversationId: conversation.id,
      conversationTitle: conversation.title,
      archiveId: conversation.archiveId,
      sender: 'Fixture Alice',
      snippet: 'Known needle',
      originalTimestamp: '04/05/2026, 09:01',
      localTimestamp: '2026-05-04T09:01:00',
    };
    vi.mocked(viewerService.search).mockResolvedValue({
      items: [hit, { ...hit, messageId: 'synthetic-message-41' }],
      nextCursor: null,
      mode: 'trigram',
      scanned: 2,
    });
    const onJump = vi.fn();
    const user = userEvent.setup();
    render(
      <ChatSearch
        conversationId={conversation.id}
        onJump={onJump}
        onClose={() => {}}
      />,
    );
    await user.selectOptions(screen.getByLabelText('Search scope'), 'all');
    await user.type(
      screen.getByRole('searchbox', { name: 'Search message text or sender' }),
      'needle',
    );
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await screen.findByText('2 results on this page');
    expect(viewerService.search).toHaveBeenCalledWith('needle', null, null);
    await user.click(
      screen.getByRole('button', { name: 'Next search result' }),
    );
    expect(onJump).toHaveBeenLastCalledWith(hit, 'needle');
    await user.click(
      screen.getByRole('button', { name: 'Next search result' }),
    );
    expect(onJump.mock.lastCall?.[0].messageId).toBe('synthetic-message-41');
    await user.click(
      screen.getByRole('button', { name: 'Previous search result' }),
    );
    expect(onJump).toHaveBeenLastCalledWith(hit, 'needle');
  });
  it('reopens the same search target on repeated selection', async () => {
    const hit = {
      messageId: 'synthetic-message-40',
      conversationId: conversation.id,
      conversationTitle: conversation.title,
      archiveId: conversation.archiveId,
      sender: 'Fixture Alice',
      snippet: 'Known needle',
      originalTimestamp: null,
      localTimestamp: null,
    };
    vi.mocked(viewerService.search).mockResolvedValue({
      items: [hit],
      nextCursor: null,
      mode: 'trigram',
      scanned: 1,
    });
    vi.mocked(viewerService.context).mockResolvedValue({
      items: [message(40, { text: 'Known needle' })],
      hasOlder: false,
      hasNewer: false,
      anchorId: hit.messageId,
    });
    const user = userEvent.setup();
    mount();
    await screen.findByText('Synthetic record 1');
    await user.click(screen.getByRole('button', { name: 'Search messages' }));
    await user.type(
      screen.getByRole('searchbox', { name: 'Search message text or sender' }),
      'needle',
    );
    await user.click(screen.getByRole('button', { name: 'Search' }));
    const result = await screen.findByRole('button', {
      name: /Synthetic conversation.*Known needle/,
    });
    await user.click(result);
    await waitFor(() => expect(viewerService.context).toHaveBeenCalledTimes(1));
    expect(document.activeElement?.id).toBe('message-synthetic-message-40');
    await user.click(result);
    await waitFor(() => expect(viewerService.context).toHaveBeenCalledTimes(2));
  });
  it('exposes bounded short-query continuation instead of claiming an incomplete empty search', async () => {
    vi.mocked(viewerService.search).mockResolvedValue({
      items: [],
      nextCursor: '2000',
      mode: 'bounded-scan',
      scanned: 2000,
    });
    const user = userEvent.setup();
    render(
      <ChatSearch conversationId={null} onJump={() => {}} onClose={() => {}} />,
    );
    await user.type(screen.getByRole('searchbox'), 'ක');
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await screen.findByText(/No matches in this scanned section/);
    await user.click(screen.getByRole('button', { name: 'Continue search' }));
    expect(viewerService.search).toHaveBeenLastCalledWith('ක', null, '2000');
  });
  it('renders literal search metacharacters and Unicode without executing markup', () => {
    const { container } = render(
      <Highlight text={'ආයුබෝවන් [a.*] <img src=x>'} query="[a.*]" />,
    );
    expect(container.querySelector('mark')).toHaveTextContent('[a.*]');
    expect(container.querySelector('img')).toBeNull();
  });
  it('highlights canonically equivalent German text while preserving the original characters', () => {
    const { container } = render(
      <Highlight text={'Gru\u0308ße'} query="GRÜßE" />,
    );
    expect(container.querySelector('mark')?.textContent).toBe('Gru\u0308ße');
  });
  it('reads long messages in bounded text sections without mounting all text or highlights', async () => {
    const user = userEvent.setup();
    const text = 'x'.repeat(9000) + 'SYNTHETIC-END';
    const { container } = render(
      <MessageBubble
        message={message(1, { text })}
        query="x"
        onEnlarge={() => {}}
      />,
    );
    expect(container.querySelector('.message-body')!.textContent!.length).toBe(
      4096,
    );
    await user.click(screen.getByRole('button', { name: 'Next text section' }));
    await user.click(screen.getByRole('button', { name: 'Next text section' }));
    expect(screen.getByText(/SYNTHETIC-END/)).toBeInTheDocument();
    expect(container.querySelectorAll('mark').length).toBeLessThanOrEqual(200);
  });
});
describe('private attachments', () => {
  it('does not authorize or load images until explicitly requested, then opens an enlargement action', async () => {
    const user = userEvent.setup();
    const enlarge = vi.fn();
    const { unmount } = render(
      <MediaMessage attachment={imageFixture} onEnlarge={enlarge} />,
    );
    expect(viewerService.authorize).not.toHaveBeenCalled();
    await user.click(
      screen.getByRole('button', { name: 'Load image preview' }),
    );
    const image = await screen.findByRole('img', { name: 'Synthetic.png' });
    expect(image).toHaveAttribute(
      'src',
      'http://chatvault-media.localhost/synthetic-private-token',
    );
    await user.click(
      screen.getByRole('button', { name: 'Enlarge Synthetic.png' }),
    );
    expect(enlarge).toHaveBeenCalledWith('synthetic-media');
    unmount();
    expect(viewerService.revoke).toHaveBeenCalledWith(
      'synthetic-private-token',
    );
  });
  it('shows missing media without a filesystem request', () => {
    render(
      <MediaMessage
        attachment={{ ...imageFixture, mediaId: null, missing: true }}
      />,
    );
    expect(screen.getByText(/omitted or missing/)).toBeInTheDocument();
    expect(viewerService.authorize).not.toHaveBeenCalled();
  });
  it('reports invalid images and unsupported codecs with retry', async () => {
    const user = userEvent.setup();
    render(<MediaMessage attachment={imageFixture} />);
    await user.click(
      screen.getByRole('button', { name: 'Load image preview' }),
    );
    fireEvent.error(await screen.findByRole('img'));
    expect(screen.getByRole('alert')).toHaveTextContent('could not be decoded');
    expect(
      screen.getByRole('button', { name: 'Retry attachment' }),
    ).toBeInTheDocument();
  });
  it('uses native video controls and shows codec failure', async () => {
    vi.mocked(viewerService.authorize).mockResolvedValue({
      ...mediaAccessFixture,
      metadata: {
        ...mediaAccessFixture.metadata,
        mime: 'video/mp4',
        kind: 'video',
      },
    });
    const user = userEvent.setup();
    const { container } = render(
      <MediaMessage
        attachment={{
          ...imageFixture,
          filename: 'Synthetic.mp4',
          mediaType: 'video/mp4',
        }}
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Load recording' }));
    await waitFor(() =>
      expect(container.querySelector('video')).toBeInTheDocument(),
    );
    expect(container.querySelector('video')).toHaveAttribute('controls');
    fireEvent.error(container.querySelector('video')!);
    expect(screen.getByRole('alert')).toHaveTextContent(
      'codec may be unsupported',
    );
  });
  it('provides local audio controls and playback speed', async () => {
    vi.mocked(viewerService.authorize).mockResolvedValue({
      ...mediaAccessFixture,
      metadata: {
        ...mediaAccessFixture.metadata,
        mime: 'audio/wav',
        kind: 'audio',
      },
    });
    const user = userEvent.setup();
    const { container } = render(
      <MediaMessage
        attachment={{
          ...imageFixture,
          filename: 'Synthetic.wav',
          mediaType: 'audio/wav',
        }}
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Load recording' }));
    const speed = await screen.findByLabelText(
      'Playback speed for Synthetic.wav',
    );
    await user.selectOptions(speed, '1.5');
    expect(container.querySelector('audio')?.playbackRate).toBe(1.5);
    expect(container.querySelector('audio')).toHaveAttribute(
      'preload',
      'metadata',
    );
  });
  it('opens documents only on an explicit user action', async () => {
    vi.mocked(viewerService.openDocument).mockResolvedValue();
    const user = userEvent.setup();
    render(
      <MediaMessage
        attachment={{
          ...imageFixture,
          mediaType: 'application/pdf',
          filename: 'Synthetic.pdf',
        }}
      />,
    );
    expect(viewerService.openDocument).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Open document' }));
    expect(viewerService.openDocument).toHaveBeenCalledWith('synthetic-media');
  });
  it('opens an accessible image dialog with zoom/fit/close controls', async () => {
    const close = vi.fn();
    const user = userEvent.setup();
    render(
      <MediaLightbox
        items={[imageFixture]}
        index={0}
        onChange={() => {}}
        onClose={close}
      />,
    );
    const dialog = screen.getByRole('dialog', { name: 'Synthetic.png' });
    await within(dialog).findByRole('img');
    await user.click(within(dialog).getByRole('button', { name: 'Zoom in' }));
    expect(within(dialog).getByText('125%')).toBeInTheDocument();
    await user.click(
      within(dialog).getByRole('button', { name: 'Fit image to screen' }),
    );
    expect(within(dialog).getByText('100%')).toBeInTheDocument();
    await user.click(
      within(dialog).getByRole('button', { name: 'Close image viewer' }),
    );
    expect(close).toHaveBeenCalled();
  });
  it('uses the same neutral bubble structure in light/dark themes', () => {
    for (const theme of ['light', 'dark']) {
      document.documentElement.dataset.theme = theme;
      const { unmount } = render(
        <MessageBubble
          message={message(0, { sender: null })}
          query=""
          onEnlarge={() => {}}
        />,
      );
      expect(screen.getByText('Unknown sender')).toBeInTheDocument();
      unmount();
    }
  });
});
