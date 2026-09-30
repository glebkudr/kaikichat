import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: { host: '127.0.0.1', port: 1420, strictPort: true, fs: {allow: [
    '.', '../../integrations/agent-skill/agentic-messaging',
  ]} },
  test: { environment: 'jsdom', setupFiles: ['./tests/setup.ts'], restoreMocks: true, pool: 'threads', maxWorkers: 1 },
});
