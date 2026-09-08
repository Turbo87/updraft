import type { EnrouteCatalogStatus } from '$lib/client';

export class EnrouteCatalogStore {
  current = $state.raw<EnrouteCatalogStatus | null>(null);
  error = $state(false);
}
