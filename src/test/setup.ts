import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, beforeEach, vi } from 'vitest';

const listeners = new Set<(event: MediaQueryListEvent) => void>();
let dark = false;
export function setSystemDark(value: boolean) {
  dark = value;
  listeners.forEach((listener) =>
    listener({ matches: value } as MediaQueryListEvent),
  );
}
beforeEach(() => {
  dark = false;
  listeners.clear();
  localStorage.clear();
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: vi.fn((query: string) => ({
      matches: dark,
      media: query,
      onchange: null,
      addEventListener: (
        _type: string,
        listener: (event: MediaQueryListEvent) => void,
      ) => listeners.add(listener),
      removeEventListener: (
        _type: string,
        listener: (event: MediaQueryListEvent) => void,
      ) => listeners.delete(listener),
    })),
  });
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  document.documentElement.removeAttribute('data-theme');
});
