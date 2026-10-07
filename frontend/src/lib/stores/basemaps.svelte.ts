import type { BasemapStatus } from '#lib/client/index.js';

export class BasemapsStore {
  current = $state.raw<BasemapStatus | null>(null);
  error = $state(false);
}
