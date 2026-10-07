import type { EnrouteDownloadStatus } from '#lib/client/index.js';

export class EnrouteDownloadsStore {
  current = $state.raw<EnrouteDownloadStatus[] | null>(null);
  error = $state(false);
}
