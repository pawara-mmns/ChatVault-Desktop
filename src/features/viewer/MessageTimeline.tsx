import { Fragment, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { ArrowDown, ChevronUp, ChevronDown } from 'lucide-react';
import {
  viewerService,
  viewerFailure,
  messageTime,
  messageDay,
  type SavedMessage,
  type MessagePage,
} from '../../services/viewer';
import { Button } from '../../shared/components/ui/button';
import { Highlight } from './Highlight';
import { MediaMessage } from './MediaMessage';
export const MAX_WINDOW_MESSAGES = 120;
export function MessageBubble({
  message,
  query,
  onEnlarge,
}: {
  message: SavedMessage;
  query: string;
  onEnlarge: (id: string) => void;
}) {
  const system = message.classification === 'system';
  return (
    <article
      id={`message-${message.id}`}
      tabIndex={-1}
      className={`message-record ${system ? 'system-record' : ''} ${message.classification === 'deleted' ? 'deleted-record' : ''}`}
      aria-label={`${system ? 'System' : (message.sender ?? 'Unknown sender')} message`}
    >
      <header>
        <strong>
          {system ? 'System' : (message.sender ?? 'Unknown sender')}
        </strong>
        {message.classification === 'unknown' && (
          <span>Unclassified record</span>
        )}
      </header>
      <MessageText key={query} text={message.text} query={query} />
      {message.attachments.map((attachment) => (
        <MediaMessage
          key={attachment.id}
          attachment={attachment}
          onEnlarge={onEnlarge}
        />
      ))}
      <time
        title={message.originalTimestamp ?? 'Original timestamp unavailable'}
      >
        {messageTime(message)}
      </time>
    </article>
  );
}
function MessageText({ text, query }: { text: string; query: string }) {
  const [start, setStart] = useState(() => {
    const found = query
      ? text.toLowerCase().indexOf(query.trim().toLowerCase())
      : -1;
    let first = Math.max(0, found - 200);
    if (
      first > 0 &&
      text.charCodeAt(first) >= 0xdc00 &&
      text.charCodeAt(first) <= 0xdfff
    )
      first -= 1;
    return first;
  });
  let end = Math.min(text.length, start + 4096);
  if (
    end < text.length &&
    text.charCodeAt(end - 1) >= 0xd800 &&
    text.charCodeAt(end - 1) <= 0xdbff
  )
    end += 1;
  return (
    <>
      <p className="message-body">
        <Highlight text={text.slice(start, end)} query={query} />
      </p>
      {text.length > 4096 && (
        <div className="reader-pagination">
          <Button
            variant="ghost"
            size="sm"
            disabled={start === 0}
            onClick={() => {
              let previous = Math.max(0, start - 4096);
              if (
                previous > 0 &&
                text.charCodeAt(previous) >= 0xdc00 &&
                text.charCodeAt(previous) <= 0xdfff
              )
                previous -= 1;
              setStart(previous);
            }}
          >
            Previous text section
          </Button>
          <span>
            {start + 1}–{end} of {text.length} text units
          </span>
          <Button
            variant="ghost"
            size="sm"
            disabled={end >= text.length}
            onClick={() => setStart(end)}
          >
            Next text section
          </Button>
        </div>
      )}
    </>
  );
}
export function MessageTimeline({
  conversationId,
  targetId,
  query,
  onEnlarge,
}: {
  conversationId: string;
  targetId: string | null;
  query: string;
  onEnlarge: (id: string) => void;
}) {
  const [page, setPage] = useState<MessagePage | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState('');
  const [revision, setRevision] = useState(0);
  const scroller = useRef<HTMLDivElement>(null);
  const active = useRef(0);
  const pending = useRef<{
    kind: 'latest' | 'anchor' | 'older' | 'newer';
    height: number;
    top: number;
    anchor?: string | null;
    visibleId?: string;
    visibleTop?: number;
  } | null>(null);
  const loading = useRef(false);
  useEffect(() => {
    let cancelled = false;
    const request = ++active.current;
    loading.current = true;
    (targetId
      ? viewerService.context(conversationId, targetId)
      : viewerService.messages(conversationId)
    )
      .then((result) => {
        if (!cancelled && request === active.current) {
          pending.current = {
            kind: targetId ? 'anchor' : 'latest',
            height: 0,
            top: 0,
            anchor: targetId,
          };
          setPage(result);
        }
      })
      .catch((e) => {
        if (!cancelled) setError(viewerFailure(e).message);
      })
      .finally(() => {
        if (!cancelled) {
          loading.current = false;
          setBusy(false);
        }
      });
    return () => {
      cancelled = true;
      active.current += 1;
    };
  }, [conversationId, targetId, revision]);
  useLayoutEffect(() => {
    const element = scroller.current;
    const action = pending.current;
    if (!element || !page || !action) return;
    pending.current = null;
    if (action.kind === 'latest') element.scrollTop = element.scrollHeight;
    else if (action.kind === 'anchor') {
      const target = document.getElementById(`message-${action.anchor}`);
      if (target)
        element.scrollTop +=
          target.getBoundingClientRect().top -
          element.getBoundingClientRect().top -
          element.clientHeight / 2 +
          target.clientHeight / 2;
      target?.focus({ preventScroll: true });
    } else if (action.visibleId && document.getElementById(action.visibleId)) {
      const record = document.getElementById(action.visibleId)!;
      element.scrollTop +=
        record.getBoundingClientRect().top - (action.visibleTop ?? 0);
    } else if (action.kind === 'older')
      element.scrollTop = Math.max(
        0,
        action.top + element.scrollHeight - action.height,
      );
    else element.scrollTop = action.top;
  }, [page]);
  async function load(direction: 'older' | 'newer' | 'latest') {
    if (!page || loading.current) return;
    loading.current = true;
    setBusy(true);
    setError('');
    const request = active.current;
    const cursor =
      direction === 'older'
        ? page.items[0]?.id
        : direction === 'newer'
          ? page.items.at(-1)?.id
          : null;
    try {
      const result = await viewerService.messages(
        conversationId,
        cursor ?? null,
        direction,
      );
      if (request !== active.current) return;
      const element = scroller.current;
      const top = element?.getBoundingClientRect().top ?? 0;
      const visible = element
        ? Array.from(
            element.querySelectorAll<HTMLElement>('.message-record'),
          ).find((record) => record.getBoundingClientRect().bottom > top)
        : undefined;
      pending.current = {
        kind: direction,
        height: element?.scrollHeight ?? 0,
        top: element?.scrollTop ?? 0,
        ...(visible
          ? {
              visibleId: visible.id,
              visibleTop: visible.getBoundingClientRect().top,
            }
          : {}),
      };
      if (direction === 'latest') setPage(result);
      else {
        const combined =
          direction === 'older'
            ? [...result.items, ...page.items]
            : [...page.items, ...result.items];
        const unique = [...new Map(combined.map((m) => [m.id, m])).values()];
        const trimmed = unique.length > MAX_WINDOW_MESSAGES;
        setPage({
          items:
            direction === 'older'
              ? unique.slice(0, MAX_WINDOW_MESSAGES)
              : unique.slice(-MAX_WINDOW_MESSAGES),
          hasOlder:
            direction === 'older' ? result.hasOlder : page.hasOlder || trimmed,
          hasNewer:
            direction === 'newer' ? result.hasNewer : page.hasNewer || trimmed,
          anchorId: null,
        });
      }
    } catch (e) {
      if (request === active.current) setError(viewerFailure(e).message);
    } finally {
      if (request === active.current) {
        loading.current = false;
        setBusy(false);
      }
    }
  }
  return (
    <div className="timeline-shell">
      <div className="timeline-tools">
        <span>
          {page
            ? `${page.items.length} records in this window`
            : 'Loading records'}
        </span>
        <Button
          size="sm"
          variant="ghost"
          disabled={busy || !page}
          onClick={() => void load('latest')}
        >
          <ArrowDown size={14} aria-hidden="true" />
          Newest available
        </Button>
      </div>
      {/* eslint-disable jsx-a11y/no-noninteractive-tabindex -- The scrollable region needs keyboard Page Up/Down access. */}
      <div
        ref={scroller}
        className="message-timeline"
        role="region"
        aria-label="Message timeline"
        tabIndex={0}
        aria-busy={busy}
      >
        {error && (
          <div className="reader-notice" role="alert">
            <p>{error}</p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setError('');
                setPage(null);
                setBusy(true);
                setRevision((v) => v + 1);
              }}
            >
              Retry messages
            </Button>
          </div>
        )}
        {busy && !page && (
          <p role="status" className="reader-notice">
            Reading saved messages…
          </p>
        )}
        {page?.hasOlder && (
          <Button
            className="timeline-page-button"
            variant="outline"
            size="sm"
            disabled={busy}
            onClick={() => void load('older')}
          >
            <ChevronUp size={15} aria-hidden="true" />
            Load older messages
          </Button>
        )}
        {page?.items.length === 0 && !busy && (
          <p className="reader-notice">
            No saved messages in this conversation.
          </p>
        )}
        {page?.items.map((message, index) => (
          <Fragment key={message.id}>
            {(index === 0 ||
              messageDay(message) !== messageDay(page.items[index - 1]!)) && (
              <div className="message-date">
                <span>{messageDay(message)}</span>
              </div>
            )}
            <div className={message.id === targetId ? 'search-target' : ''}>
              <MessageBubble
                message={message}
                query={query}
                onEnlarge={onEnlarge}
              />
            </div>
          </Fragment>
        ))}
        {page?.hasNewer && (
          <Button
            className="timeline-page-button"
            variant="outline"
            size="sm"
            disabled={busy}
            onClick={() => void load('newer')}
          >
            <ChevronDown size={15} aria-hidden="true" />
            Load newer messages
          </Button>
        )}
        {busy && page && (
          <p role="status" className="reader-notice">
            Reading the next page…
          </p>
        )}
      </div>
      {/* eslint-enable jsx-a11y/no-noninteractive-tabindex */}
      <p className="timeline-footnote">
        Read-only archive · neutral sender alignment · exported times have no
        known timezone
      </p>
    </div>
  );
}
