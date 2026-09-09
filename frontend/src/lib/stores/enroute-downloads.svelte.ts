import type { EnrouteDownloadStatus } from '$lib/client';

export class EnrouteDownloadsStore {
  current = $state.raw<EnrouteDownloadStatus[] | null>(null);
  error = $state(false);
}
