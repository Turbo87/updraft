import flags from '@iconify-json/circle-flags/icons.json';
import { defineConfig, presetIcons } from 'unocss';

export default defineConfig({
  // UnoCSS's top-level-await HMR can race SvelteKit module initialization in
  // Tauri's WKWebView, leaving page components uninitialized at startup.
  hmrTopLevelAwait: false,
  presets: [presetIcons()],
  safelist: Object.keys(flags.icons)
    .filter((code) => /^[a-z]{2}$/.test(code))
    .map((code) => `i-circle-flags-${code}`),
});
