import type * as GeoJSON from 'geojson';

type AirspaceFixtureProperties = {
  class: string | null;
  type: string | null;
};

/** Fixed airspace GeoJSON for browser tests and Storybook. */
export const AIRSPACE_BROWSER_FIXTURE = {
  type: 'FeatureCollection',
  features: [
    {
      type: 'Feature',
      id: 0,
      properties: { class: 'D', type: 'CTR' },
      geometry: {
        type: 'Polygon',
        coordinates: [
          [
            [6.16, 50.81],
            [6.185, 50.81],
            [6.185, 50.835],
            [6.16, 50.835],
            [6.16, 50.81],
          ],
        ],
      },
    },
    {
      type: 'Feature',
      id: 1,
      properties: { class: 'E', type: 'RMZ' },
      geometry: {
        type: 'Polygon',
        coordinates: [
          [
            [6.187, 50.81],
            [6.212, 50.81],
            [6.212, 50.835],
            [6.187, 50.835],
            [6.187, 50.81],
          ],
        ],
      },
    },
  ],
} satisfies GeoJSON.FeatureCollection<GeoJSON.Polygon, AirspaceFixtureProperties>;
