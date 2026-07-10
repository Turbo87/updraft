import { defineConfig, devices } from '@playwright/test';

// End-to-end tests exercise the real SvelteKit router, so they need the built app
// served, not a mounted component. The reducer's pure logic is unit-tested in
// src/lib/dialog/navigation.test.ts; these tests cover the wiring (history.go,
// popstate, cold deep-links) against a real browser history stack.
const PORT = 4173;

export default defineConfig({
  testDir: 'e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  use: {
    baseURL: `http://localhost:${PORT}`,
    trace: 'on-first-retry',
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // Portable by default (Playwright resolves its own browser). Set
        // PLAYWRIGHT_CHROMIUM_PATH to reuse a pre-installed Chromium binary.
        launchOptions: { executablePath: process.env.PLAYWRIGHT_CHROMIUM_PATH || undefined },
      },
    },
  ],
  webServer: {
    command: `pnpm build && pnpm preview --port ${PORT} --strictPort`,
    port: PORT,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
