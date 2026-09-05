import type { FeatureCollection, Point } from 'geojson';

export const waypointsFixture: FeatureCollection<Point> = {
  type: 'FeatureCollection',
  features: [2, 3, 7].map((kind, index) => ({
    type: 'Feature',
    id: `1:0:${index}`,
    geometry: { type: 'Point', coordinates: [6.186 + index * 0.005, 50.823] },
    properties: {
      id: `1:0:${index}`,
      name: `Point ${index}`,
      sourceName: 'local.cup',
      kind,
      elevationMeters: 100,
      runwayDirection: 90,
      runwayLengthMeters: 800,
      frequency: '123.500',
      notes: 'Notes',
    },
  })),
};
