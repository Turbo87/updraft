import { execFileSync } from 'node:child_process';

import { paraglideVitePlugin } from '@inlang/paraglide-js';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { playwright } from '@vitest/browser-playwright';
import { browserslistToTargets } from 'lightningcss';
import UnoCSS from 'unocss/vite';
import { defineConfig } from 'vitest/config';

const cssTargets = browserslistToTargets(['chrome 87', 'android 87', 'safari 14']);
const buildCommitSha = resolveBuildCommitSha();
const buildTimestamp = new Date().toISOString();

function resolveBuildCommitSha(): string | undefined {
  try {
    return execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  } catch {
    return undefined;
  }
}

export default defineConfig({
  define: {
    __BUILD_COMMIT_SHA__:
      buildCommitSha === undefined ? 'undefined' : JSON.stringify(buildCommitSha),
    __BUILD_TIMESTAMP__: JSON.stringify(buildTimestamp),
  },
  css: {
    transformer: 'lightningcss',
    lightningcss: {
      targets: cssTargets,
    },
  },
  build: {
    cssMinify: 'lightningcss',
  },
  optimizeDeps: {
    // This package contains a Vite worker import that breaks when prebundled.
    exclude: ['svelte-maplibre-gl'],
  },
  plugins: [
    UnoCSS(),
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: './src/lib/paraglide',
      strategy: ['preferredLanguage', 'baseLocale'],
    }),
    sveltekit({
      compilerOptions: {
        // Force runes mode for the project, except for libraries. Can be removed in svelte 6.
        runes: ({ filename }) =>
          filename.split(/[/\\]/).includes('node_modules') ? undefined : true,
      },
      adapter: adapter({ fallback: 'index.html' }),
    }),
  ],
  server: {
    host: process.env.TAURI_DEV_HOST,
  },
  test: {
    expect: { requireAssertions: true },
    projects: [
      {
        extends: './vite.config.ts',
        test: {
          name: 'client',
          browser: {
            enabled: true,
            provider: playwright(),
            instances: [{ browser: 'chromium', headless: true }],
          },
          include: ['src/**/*.svelte.{test,spec}.{js,ts}'],
          exclude: ['src/lib/server/**'],
        },
      },

      {
        extends: './vite.config.ts',
        test: {
          name: 'server',
          environment: 'node',
          include: ['src/**/*.{test,spec}.{js,ts}'],
          exclude: ['src/**/*.svelte.{test,spec}.{js,ts}'],
        },
      },
    ],
  },
});
