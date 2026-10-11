/** A value and whether it comes from the fallback method. */
export type PreferredValue<T> = { value: T; fallback: boolean };

/**
 * Chooses between the values of two methods in the order fresh best value,
 * fresh fallback, stale best value, then stale fallback.
 */
export function preferredValue<T extends { stale: boolean }>(
  best: T | null,
  fallback: T | null,
): PreferredValue<T> | null {
  if (best && !best.stale) return { value: best, fallback: false };
  if (fallback && !fallback.stale) return { value: fallback, fallback: true };
  if (best) return { value: best, fallback: false };
  if (fallback) return { value: fallback, fallback: true };
  return null;
}
