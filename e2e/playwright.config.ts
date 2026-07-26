import { defineConfig } from '@playwright/test';

const PORT = 4450;
const HOST = '127.0.0.1';
const BASE_URL = `http://${HOST}:${PORT}`;

export default defineConfig({
  testDir: './tests',
  use: {
    baseURL: BASE_URL,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  webServer: {
    // `--host` is required: vite preview otherwise binds ::1 only, which the
    // IPv4 `url` below can never reach.
    command: `pnpm --filter @updraft/frontend preview --port ${PORT} --strictPort --host ${HOST}`,
    cwd: '..',
    gracefulShutdown: { signal: 'SIGINT', timeout: 5_000 },
    reuseExistingServer: false,
    timeout: 120_000,
    url: BASE_URL,
  },
});
