import type { TerrainStatus } from '#lib/client/index.js';

export class TerrainStore {
  current = $state.raw<TerrainStatus | null>(null);
  error = $state(false);
}
