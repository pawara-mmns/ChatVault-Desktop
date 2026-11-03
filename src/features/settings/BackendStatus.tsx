import { useCallback, useEffect, useRef, useState } from 'react';
import { CircleCheck, CircleAlert, RotateCw } from 'lucide-react';
import { checkBackend, type BackendHealth } from '../../platform/backend';
import { Button } from '../../shared/components/ui/button';
type HealthState =
  | { kind: 'checking' }
  | { kind: 'ok'; health: BackendHealth }
  | { kind: 'error'; message: string };
export function BackendStatus() {
  const [state, setState] = useState<HealthState>({ kind: 'checking' });
  const attempt = useRef(0);
  const check = useCallback(async () => {
    const id = ++attempt.current;
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      const health = await Promise.race([
        checkBackend(),
        new Promise<never>((_, reject) => {
          timer = setTimeout(() => reject(new Error('Backend timeout')), 5000);
        }),
      ]);
      if (attempt.current === id) setState({ kind: 'ok', health });
    } catch (error) {
      if (attempt.current === id)
        setState({
          kind: 'error',
          message:
            error instanceof Error &&
            error.message.startsWith('Browser preview:')
              ? error.message
              : 'The desktop backend could not be reached. Please retry or restart the app.',
        });
    } finally {
      clearTimeout(timer);
    }
  }, []);
  useEffect(() => {
    // Defer the initial IPC request; cancel the scheduled work on unmount/StrictMode cleanup.
    const scheduled = setTimeout(() => {
      void check();
    }, 0);
    return () => {
      clearTimeout(scheduled);
      attempt.current += 1;
    };
  }, [check]);
  return (
    <div className="backend-status">
      <div aria-live="polite" role="status" className="backend-message">
        {state.kind === 'ok' ? (
          <CircleCheck size={18} className="text-primary" aria-hidden="true" />
        ) : (
          <CircleAlert size={18} aria-hidden="true" />
        )}
        <div>
          <strong>
            {state.kind === 'ok'
              ? 'Desktop backend connected'
              : state.kind === 'checking'
                ? 'Checking desktop backend…'
                : 'Desktop backend unavailable'}
          </strong>
          <p>
            {state.kind === 'ok'
              ? `Rust · ${state.health.platform} · v${state.health.version}`
              : state.kind === 'error'
                ? state.message
                : 'Waiting for a response from Rust.'}
          </p>
        </div>
      </div>
      <Button
        variant="outline"
        size="sm"
        onClick={() => {
          setState({ kind: 'checking' });
          void check();
        }}
        disabled={state.kind === 'checking'}
      >
        <RotateCw size={14} aria-hidden="true" />
        Retry check
      </Button>
    </div>
  );
}
