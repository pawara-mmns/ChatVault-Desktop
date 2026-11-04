import { useEffect, useState } from 'react';
import {
  archiveService,
  importFailure,
  type ArchivePage,
} from '../../services/imports';
import type { StorageFailure } from '../../services/storage';
import { useStorage } from '../storage/StorageProvider';

export function useArchives(
  query = '',
  offset = 0,
  limit = 20,
  enabled = true,
) {
  const { overview } = useStorage();
  const [page, setPage] = useState<ArchivePage | null>(null);
  const [loading, setLoading] = useState(enabled);
  const [error, setError] = useState<StorageFailure | null>(null);
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let active = true;
    const scheduled = setTimeout(
      () => {
        if (!enabled) {
          setPage(null);
          setLoading(false);
          return;
        }
        setLoading(true);
        setError(null);
        void archiveService
          .list(query, offset, limit)
          .then((result) => {
            if (!result || !Array.isArray(result.items))
              throw new Error('Invalid archive response');
            if (active) setPage(result);
          })
          .catch((failure) => {
            if (active) {
              setError(importFailure(failure));
              setPage(null);
            }
          })
          .finally(() => {
            if (active) setLoading(false);
          });
      },
      query ? 180 : 0,
    );
    return () => {
      active = false;
      clearTimeout(scheduled);
    };
  }, [query, offset, limit, enabled, overview?.chatArchives, revision]);
  return {
    page,
    loading,
    error,
    refresh: () => setRevision((value) => value + 1),
  };
}
