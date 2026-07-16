import type { Locale } from '$lib/paraglide/runtime.js';

import {
  getLocale,
  overwriteGetLocale,
  setLocale as persistLocale,
} from '$lib/paraglide/runtime.js';

// Mirror the active locale in reactive state. Every `m.*()` message and any
// `getLocale()` call now reads this signal, so Svelte re-runs them when the
// locale changes and swaps the translated strings in place — no page reload.
let locale = $state<Locale>(getLocale());
overwriteGetLocale(() => locale);

/**
 * Switch the active locale without a full page reload.
 *
 * Updates the reactive locale (re-rendering all translated strings) and
 * persists the choice through Paraglide's configured strategies.
 */
export function setLocale(newLocale: Locale): void {
  locale = newLocale;
  persistLocale(newLocale, { reload: false });
}
