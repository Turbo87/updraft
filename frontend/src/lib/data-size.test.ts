import { afterEach, expect, it, vi } from 'vitest';

import { formatDataSize } from './data-size';
import { getLocale } from './paraglide/runtime.js';

vi.mock('./paraglide/runtime.js', () => ({ getLocale: vi.fn(() => 'en') }));
afterEach(() => vi.mocked(getLocale).mockReturnValue('en'));

it('formats decimal megabytes with one fractional digit and nonbreaking spaces', () => {
  expect(formatDataSize(0)).toBe('0\u00a0MB');
  expect(formatDataSize(1_250_000)).toBe('1.3\u00a0MB');
  expect(formatDataSize(1_000_000_000)).toBe('1,000\u00a0MB');
});

it('reads the current locale on each call', () => {
  expect(formatDataSize(1_250_000)).toBe('1.3\u00a0MB');
  vi.mocked(getLocale).mockReturnValue('de');
  expect(formatDataSize(1_250_000)).toBe('1,3\u00a0MB');
});
