import type { Locale } from '$lib/protocol/generated/Locale';

import { getLocale, overwriteGetLocale } from '$lib/paraglide/runtime.js';

const automaticLocale: Locale = getLocale();
let locale = $state<Locale>(automaticLocale);
overwriteGetLocale(() => locale);

export function applyLocaleSetting(configured: Locale | null): void {
  locale = configured ?? automaticLocale;
}
