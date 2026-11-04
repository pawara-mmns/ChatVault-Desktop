import { useEffect, useState } from 'react';
import { Search, MessageSquare, ChevronLeft, ChevronRight } from 'lucide-react';
import {
  viewerService,
  viewerFailure,
  messageTime,
  type ConversationPage,
} from '../../services/viewer';
import { Button } from '../../shared/components/ui/button';
export function ConversationSidebar({
  selected,
  onSelect,
}: {
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const [query, setQuery] = useState('');
  const [offset, setOffset] = useState(0);
  const [page, setPage] = useState<ConversationPage | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let active = true;
    const timer = setTimeout(() => {
      setLoading(true);
      setError('');
      viewerService
        .conversations(query, null, offset, 30)
        .then((p) => {
          if (active) setPage(p);
        })
        .catch((e) => {
          if (active) setError(viewerFailure(e).message);
        })
        .finally(() => {
          if (active) setLoading(false);
        });
    }, 150);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [query, offset, retry]);
  return (
    <aside className="conversation-sidebar" aria-label="Conversations">
      <div className="reader-section-title">
        <h2>Conversations</h2>
        <span>{page?.total ?? '—'}</span>
      </div>
      <label className="reader-filter">
        <Search size={15} aria-hidden="true" />
        <input
          type="search"
          aria-label="Filter conversations"
          placeholder="Find a conversation"
          value={query}
          maxLength={200}
          onChange={(e) => {
            setQuery(e.target.value);
            setOffset(0);
          }}
        />
      </label>
      <p className="reader-caption">
        Latest known local date first. Unknown dates follow import order.
      </p>
      {loading && (
        <p role="status" className="reader-notice">
          Loading conversations…
        </p>
      )}
      {error && (
        <div role="alert" className="reader-notice">
          <p>{error}</p>
          <Button
            size="sm"
            variant="outline"
            onClick={() => setRetry((v) => v + 1)}
          >
            Retry conversations
          </Button>
        </div>
      )}
      {!loading && !error && page?.items.length === 0 && (
        <div className="reader-notice">
          <MessageSquare aria-hidden="true" />
          <p>
            {query
              ? 'No matching conversations.'
              : 'Import a chat to start reading.'}
          </p>
        </div>
      )}
      <nav className="conversation-list" aria-label="Saved conversations">
        {!error &&
          page?.items.map((item) => (
            <button
              key={item.id}
              className={`conversation-item ${selected === item.id ? 'is-selected' : ''}`}
              aria-current={selected === item.id ? 'true' : undefined}
              onClick={() => onSelect(item.id)}
            >
              <span className="conversation-initial" aria-hidden="true">
                {Array.from(item.title)[0]?.toLocaleUpperCase() ?? 'C'}
              </span>
              <span className="conversation-copy">
                <strong>{item.title}</strong>
                <span>{item.lastPreview ?? 'No message text available'}</span>
                <small>
                  {messageTime({
                    localTimestamp: item.lastLocalTimestamp,
                    originalTimestamp: item.lastOriginalTimestamp,
                  })}
                </small>
              </span>
              <span
                className="conversation-count"
                aria-label={`${item.messageCount} records`}
              >
                {item.messageCount.toLocaleString()}
              </span>
            </button>
          ))}
      </nav>
      {page && page.total > 30 && (
        <div className="reader-pagination">
          <Button
            size="icon"
            variant="ghost"
            aria-label="Previous conversations"
            disabled={loading || offset === 0}
            onClick={() => setOffset((v) => Math.max(0, v - 30))}
          >
            <ChevronLeft size={16} />
          </Button>
          <span>
            {offset + 1}–{Math.min(offset + 30, page.total)}
          </span>
          <Button
            size="icon"
            variant="ghost"
            aria-label="Next conversations"
            disabled={loading || offset + 30 >= page.total}
            onClick={() => setOffset((v) => v + 30)}
          >
            <ChevronRight size={16} />
          </Button>
        </div>
      )}
    </aside>
  );
}
