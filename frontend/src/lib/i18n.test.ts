import { describe, expect, it } from 'vitest';
import { m } from '$lib/paraglide/messages.js';
import { baseLocale, locales } from '$lib/paraglide/runtime.js';

describe('i18n', () => {
	it('falls back to English as the base locale', () => {
		expect(baseLocale).toBe('en');
		expect(locales).toEqual(['en', 'de']);
	});

	it('translates messages per locale', () => {
		expect(m.brand_tagline({}, { locale: 'en' })).toBe('Soaring flight computer');
		expect(m.brand_tagline({}, { locale: 'de' })).toBe('Segelflugrechner');
	});
});
