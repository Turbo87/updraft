import { defineConfig, presetIcons } from 'unocss';

export default defineConfig({
  // UnoCSS's top-level-await HMR can race SvelteKit module initialization in
  // Tauri's WKWebView, leaving page components uninitialized at startup.
  hmrTopLevelAwait: false,
  presets: [presetIcons()],
});
