import type { EnrouteCatalogEntry } from './client';

export function catalogEntries(): EnrouteCatalogEntry[] {
  return [
    {
      path: 'Europe/Germany.mbtiles',
      countryCode: 'DE',
      continent: 'europe',
      size: 10,
      publicationDate: '2026-09-08',
    },
    {
      path: 'Europe/France/North.mbtiles',
      countryCode: 'FR',
      continent: 'europe',
      size: 20,
      publicationDate: '2026-09-08',
    },
    {
      path: 'Europe/France/South.mbtiles',
      countryCode: 'FR',
      continent: 'europe',
      size: 30,
      publicationDate: '2026-09-08',
    },
    {
      path: 'Africa/Morocco.mbtiles',
      countryCode: 'MA',
      continent: 'africa',
      size: 40,
      publicationDate: '2026-09-08',
    },
  ];
}
