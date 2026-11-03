import {
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react';
export type ThemePreference = 'light' | 'dark' | 'system';
export const THEME_STORAGE_KEY = 'chatvault.appearance.v1';
const QUERY = '(prefers-color-scheme: dark)';
function readPreference(): ThemePreference {
  try {
    const saved = localStorage.getItem(THEME_STORAGE_KEY);
    return saved === 'light' || saved === 'dark' ? saved : 'system';
  } catch {
    return 'system';
  }
}
interface ThemeContextValue {
  preference: ThemePreference;
  resolvedTheme: 'light' | 'dark';
  setPreference: (theme: ThemePreference) => void;
  persistenceError: boolean;
}
const ThemeContext = createContext<ThemeContextValue | null>(null);
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [preference, setTheme] = useState<ThemePreference>(readPreference);
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia(QUERY).matches,
  );
  const [persistenceError, setPersistenceError] = useState(false);
  const resolvedTheme =
    preference === 'system' ? (systemDark ? 'dark' : 'light') : preference;
  useEffect(() => {
    const media = window.matchMedia(QUERY);
    const onChange = (event: MediaQueryListEvent) =>
      setSystemDark(event.matches);
    media.addEventListener('change', onChange);
    return () => media.removeEventListener('change', onChange);
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme;
    document.documentElement.style.colorScheme = resolvedTheme;
  }, [resolvedTheme]);
  const value = useMemo(
    () => ({
      preference,
      resolvedTheme,
      persistenceError,
      setPreference(theme: ThemePreference) {
        setTheme(theme);
        try {
          localStorage.setItem(THEME_STORAGE_KEY, theme);
          setPersistenceError(false);
        } catch {
          setPersistenceError(true);
        }
      },
    }),
    [preference, resolvedTheme, persistenceError],
  );
  return (
    <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
  );
}
export function useTheme() {
  const context = useContext(ThemeContext);
  if (!context) throw new Error('useTheme requires ThemeProvider');
  return context;
}
