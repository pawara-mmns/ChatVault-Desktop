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
  deviceService,
  deviceFailure,
  type ConnectedDevice,
  type DeviceFailure,
  type DeviceList,
  type DeviceService,
} from '../../services/devices';
const empty: DeviceList = {
  devices: [],
  scannedAt: null,
  stale: true,
  busy: false,
};
interface DevicesContext {
  list: DeviceList;
  busy: string | null;
  error: DeviceFailure | null;
  available: boolean;
  refresh(): Promise<void>;
  inspect(id: string, discovery: boolean): Promise<void>;
  cancel(): Promise<void>;
  service: DeviceService;
}
const Context = createContext<DevicesContext>({
  list: empty,
  busy: null,
  error: null,
  available: false,
  refresh: async () => {},
  inspect: async () => {},
  cancel: async () => {},
  service: deviceService,
});
export function DeviceProvider({
  children,
  service = deviceService,
  enabled = true,
}: {
  children: ReactNode;
  service?: DeviceService;
  enabled?: boolean;
}) {
  const [list, setList] = useState(empty);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<DeviceFailure | null>(null);
  const running = useRef(false);
  const generation = useRef(0);
  const available = service.available();
  const refresh = useCallback(async () => {
    if (!available || !enabled || running.current) return;
    running.current = true;
    const ticket = ++generation.current;
    setBusy('Scanning portable devices');
    setError(null);
    setList((old) => ({ ...old, devices: [], stale: true }));
    try {
      const result = await service.refresh();
      if (ticket === generation.current) setList({ ...result, busy: false });
    } catch (e) {
      if (ticket === generation.current) {
        setError(deviceFailure(e));
        setList(empty);
      }
    } finally {
      running.current = false;
      if (ticket === generation.current) setBusy(null);
    }
  }, [available, service, enabled]);
  const inspect = useCallback(
    async (id: string, discovery: boolean) => {
      if (running.current) return;
      running.current = true;
      const ticket = ++generation.current;
      setBusy(
        discovery ? 'Discovering WhatsApp storage' : 'Checking file access',
      );
      setError(null);
      try {
        const result: ConnectedDevice = await (discovery
          ? service.discover(id)
          : service.check(id));
        if (ticket === generation.current)
          setList((old) => ({
            ...old,
            devices: old.devices.map((d) => (d.id === id ? result : d)),
          }));
      } catch (e) {
        if (ticket === generation.current) {
          const failure = deviceFailure(e);
          setError(failure);
          if (['disconnected', 'invalid_handle'].includes(failure.code))
            setList((old) => ({
              ...old,
              devices: old.devices.filter((d) => d.id !== id),
            }));
        }
      } finally {
        running.current = false;
        if (ticket === generation.current) setBusy(null);
      }
    },
    [service],
  );
  const cancel = useCallback(async () => {
    try {
      await service.cancel();
    } catch (e) {
      setError(deviceFailure(e));
    }
  }, [service]);
  const invalidate = useCallback(() => {
    generation.current++;
  }, []);
  useEffect(() => {
    if (!available || !enabled) return;
    let disposed = false;
    const scan = () => {
      if (!disposed && document.visibilityState !== 'hidden') void refresh();
    };
    scan();
    const timer = window.setInterval(scan, 15000);
    const visibility = () => {
      if (document.visibilityState === 'hidden') {
        invalidate();
        setList(empty);
        setBusy(null);
        void service.cancel().catch(() => {});
      } else scan();
    };
    document.addEventListener('visibilitychange', visibility);
    window.addEventListener('focus', scan);
    return () => {
      disposed = true;
      invalidate();
      window.clearInterval(timer);
      document.removeEventListener('visibilitychange', visibility);
      window.removeEventListener('focus', scan);
      void service.cancel().catch(() => {});
    };
  }, [available, refresh, service, invalidate, enabled]);
  return (
    <Context.Provider
      value={{
        list,
        busy,
        error,
        available,
        refresh,
        inspect,
        cancel,
        service,
      }}
    >
      {children}
    </Context.Provider>
  );
}
export function useDevices() {
  return useContext(Context);
}
