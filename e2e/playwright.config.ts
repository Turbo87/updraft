import { defineConfig } from '@playwright/test';

const PORT = 4450;
const BASE_URL = `http://127.0.0.1:${PORT}`;

export default defineConfig({
  testDir: './tests',
  use: {
    baseURL: BASE_URL,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
  },
  webServer: {
    command: `cargo run -p updraft_server -- --port ${PORT} --simulation --static-dir frontend/build`,
    cwd: '..',
    gracefulShutdown: { signal: 'SIGINT', timeout: 5_000 },
    reuseExistingServer: false,
    timeout: 120_000,
    url: `${BASE_URL}/api/health`,
  },
});
