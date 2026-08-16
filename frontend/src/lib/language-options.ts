import type { Locale } from '$lib/protocol/generated/Locale';

// @unocss-include
export const languageOptions = [
  { locale: 'en', label: 'English', icon: 'i-circle-flags-lang-en' },
  { locale: 'de', label: 'Deutsch', icon: 'i-circle-flags-lang-de' },
] satisfies Array<{ locale: Locale; label: string; icon: string }>;
