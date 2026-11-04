import { Link } from 'react-router-dom';
import { useStorage } from '../storage/StorageProvider';
import { useArchives } from './useArchives';
import { formatBytes } from '../../services/storage';

export function RecentArchives() {
  const { overview } = useStorage();
  const { page, loading, error } = useArchives(
    '',
    0,
    3,
    (overview?.chatArchives ?? 0) > 0,
  );
  if (!overview || overview.chatArchives === 0) return null;
  return (
    <section
      className="panel recent-archives"
      aria-labelledby="recent-archives-heading"
    >
      <div className="panel-heading">
        <h2 id="recent-archives-heading">Recent archives</h2>
        <Link to="/archives" className="text-link">
          Open library
        </Link>
      </div>
      {loading && (
        <p className="library-status" role="status">
          Loading archives…
        </p>
      )}
      {error && <p className="library-status">{error.message}</p>}
      {page?.items.map((item) => (
        <div className="recent-archive" key={item.id}>
          <div>
            <strong>{item.title}</strong>
            <p>{item.originalFilename}</p>
          </div>
          <span>
            {item.messageCount} records · {formatBytes(item.totalBytes)}
          </span>
        </div>
      ))}
    </section>
  );
}
