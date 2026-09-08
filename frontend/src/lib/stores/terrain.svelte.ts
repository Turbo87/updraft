import type { TerrainStatus } from '$lib/client';

export class TerrainStore {
  current = $state.raw<TerrainStatus | null>(null);
  error = $state(false);
}
