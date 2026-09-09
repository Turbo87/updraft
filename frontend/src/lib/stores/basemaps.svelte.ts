import type { BasemapStatus } from '$lib/client';

export class BasemapsStore {
  current = $state.raw<BasemapStatus | null>(null);
  error = $state(false);
}
