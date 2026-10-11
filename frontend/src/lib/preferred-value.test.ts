import { describe, expect, it } from 'vitest';

import { preferredValue } from './preferred-value';

const best = { metersPerSecond: 1, stale: false };
const fallback = { metersPerSecond: 2, stale: false };
const staleBest = { ...best, stale: true };
const staleFallback = { ...fallback, stale: true };

describe('preferredValue', () => {
  it.each([
    [best, fallback, { value: best, fallback: false }],
    [best, staleFallback, { value: best, fallback: false }],
    [best, null, { value: best, fallback: false }],
    [staleBest, fallback, { value: fallback, fallback: true }],
    [staleBest, staleFallback, { value: staleBest, fallback: false }],
    [staleBest, null, { value: staleBest, fallback: false }],
    [null, fallback, { value: fallback, fallback: true }],
    [null, staleFallback, { value: staleFallback, fallback: true }],
    [null, null, null],
  ])('chooses between best %j and fallback %j', (best, fallback, expected) => {
    expect(preferredValue(best, fallback)).toEqual(expected);
  });
});
