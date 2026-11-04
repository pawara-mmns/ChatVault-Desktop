import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import {
  nativeStorageService,
  storageFailure,
  type StorageFailure,
  type StorageOverview,
  type StorageService,
} from '../../services/storage';

interface StorageContextValue {
  overview: StorageOverview | null;
  loading: boolean;
  error: StorageFailure | null;
  success: string | null;
  refresh: () => Promise<void>;
  initialize: () => Promise<void>;
  changeLocation: () => Promise<void>;
}
const StorageContext = createContext<StorageContextValue | null>(null);

export function StorageProvider({
  children,
  service = nativeStorageService,
}: {
  children: ReactNode;
  service?: StorageService;
}) {
  const [overview, setOverview] = useState<StorageOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<StorageFailure | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const pending = useRef(false);
  const mounted = useRef(true);
  const run = useCallback(
    async (operation: 'refresh' | 'initialize' | 'change') => {
      if (pending.current) return;
      pending.current = true;
      setLoading(true);
      setError(null);
      setSuccess(null);
      try {
        const result = await (operation === 'change'
          ? service.chooseVaultDirectory()
          : operation === 'initialize'
            ? service.initializeVault()
            : service.getOverview());
        if (mounted.current && result) {
          setOverview(result);
          if (operation === 'change')
            setSuccess(
              'Vault location saved. Existing files were not moved or deleted.',
            );
          if (operation === 'initialize' && result.vaultStatus === 'ready')
            setSuccess('Local database and vault are ready.');
        }
      } catch (failure) {
        // A database commit can succeed even if the subsequent response fails. Re-read the
        // authoritative setting after a failed change instead of displaying a stale path.
        if (operation === 'change') {
          try {
            const latest = await service.getOverview();
            if (mounted.current) setOverview(latest);
          } catch {
            if (mounted.current) setOverview(null);
          }
        }
        if (mounted.current) {
          setError(storageFailure(failure));
          if (operation !== 'change') setOverview(null);
        }
      } finally {
        pending.current = false;
        if (mounted.current) setLoading(false);
      }
    },
    [service],
  );
  useEffect(() => {
    mounted.current = true;
    const scheduled = setTimeout(() => {
      void run('refresh');
    }, 0);
    return () => {
      mounted.current = false;
      clearTimeout(scheduled);
    };
  }, [run]);
  return (
    <StorageContext.Provider
      value={{
        overview,
        loading,
        error,
        success,
        refresh: () => run('refresh'),
        initialize: () => run('initialize'),
        changeLocation: () => run('change'),
      }}
    >
      {children}
    </StorageContext.Provider>
  );
}

export function useStorage() {
  const value = useContext(StorageContext);
  if (!value) throw new Error('Storage UI must be wrapped in StorageProvider');
  return value;
}
