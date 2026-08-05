import type * as GeoJSON from 'geojson';

type AirspaceFixtureProperties = {
  icaoClass: number;
  type: number;
};

/** Fixed airspace GeoJSON for browser tests and Storybook. */
export const AIRSPACE_BROWSER_FIXTURE = {
  type: 'FeatureCollection',
  features: [
    {
      type: 'Feature',
      id: 0,
      properties: { icaoClass: 3, type: 4 },
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
      properties: { icaoClass: 4, type: 6 },
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
