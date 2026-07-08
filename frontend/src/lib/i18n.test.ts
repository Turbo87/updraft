import { describe, expect, it } from 'vitest';
import { m } from '$lib/paraglide/messages.js';
import { baseLocale, locales } from '$lib/paraglide/runtime.js';

describe('i18n', () => {
	it('falls back to English as the base locale', () => {
		expect(baseLocale).toBe('en');
		expect(locales).toEqual(['en', 'de']);
	});

	it('translates messages per locale', () => {
		expect(m.language_label({}, { locale: 'en' })).toBe('Language');
		expect(m.language_label({}, { locale: 'de' })).toBe('Sprache');
	});
});
