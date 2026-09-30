import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';
afterEach(cleanup);
// The window keeps its language choice in localStorage; give the test
// environment one when it has none.
if (typeof globalThis.localStorage?.clear !== 'function') {
  const items = new Map<string, string>();
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: (key: string) => items.get(key) ?? null,
      setItem: (key: string, value: string) => void items.set(key, String(value)),
      removeItem: (key: string) => void items.delete(key),
      clear: () => items.clear(),
      key: (index: number) => [...items.keys()][index] ?? null,
      get length() { return items.size; },
    },
  });
}
