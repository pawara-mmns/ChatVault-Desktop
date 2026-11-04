import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  activePhase,
  backupService,
  idleProgress,
  type BackupProgress,
  type BackupService,
} from '../../services/backups';
interface Value {
  progress: BackupProgress;
  setProgress(p: BackupProgress): void;
  service: BackupService;
}
const Context = createContext<Value>({
  progress: idleProgress,
  setProgress: () => {},
  service: backupService,
});
export function BackupProvider({
  children,
  service = backupService,
}: {
  children: ReactNode;
  service?: BackupService;
}) {
  const [progress, setValue] = useState(idleProgress);
  const setProgress = useCallback((p: BackupProgress) => {
    if (p && typeof p.phase === 'string' && typeof p.totalFiles === 'number')
      setValue((old) => {
        if (old.revision)
          return p.revision && p.revision > old.revision ? p : old;
        return p.updatedAt < old.updatedAt ? old : p;
      });
  }, []);
  useEffect(() => {
    if (!service.available()) return;
    let mounted = true;
    let busy = false;
    let unlisten: (() => void) | undefined;
    const poll = async () => {
      if (busy) return;
      busy = true;
      try {
        const p = await service.progress();
        if (mounted) setProgress(p);
      } catch {
        /* The page exposes errors for explicit actions; polling never invents success. */
      } finally {
        busy = false;
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 1000);
    if (service === backupService)
      void listen<BackupProgress>('backup-progress', (event) => {
        if (mounted) setProgress(event.payload);
      })
        .then((fn) => {
          if (mounted) unlisten = fn;
          else fn();
        })
        .catch(() => {});
    return () => {
      mounted = false;
      clearInterval(timer);
      unlisten?.();
    };
  }, [service, setProgress]);
  return (
    <Context.Provider value={{ progress, setProgress, service }}>
      {children}
      {activePhase(progress.phase) && progress.snapshotId && (
        <div className="backup-global-status" role="status">
          Backup: {progress.phase.replaceAll('_', ' ')} ·{' '}
          {progress.successfulFiles}/{progress.totalFiles} files
        </div>
      )}
    </Context.Provider>
  );
}
export function useBackups() {
  return useContext(Context);
}
