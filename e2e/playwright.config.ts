import { defineConfig } from 'playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 30_000,
  use: {
    baseURL: 'http://127.0.0.1:4450',
    headless: true,
  },
  webServer: {
    // CI builds the server binary beforehand, so `cargo run` starts it
    // immediately; the generous timeout covers cold local builds.
    command:
      'cargo run --quiet -p updraft_server -- --simulation --port 4450 --static-dir frontend/build',
    cwd: '..',
    url: 'http://127.0.0.1:4450/api/health',
    timeout: 300_000,
    reuseExistingServer: false,
    gracefulShutdown: { signal: 'SIGTERM', timeout: 5_000 },
  },
});
