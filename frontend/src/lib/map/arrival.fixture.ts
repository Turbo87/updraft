import type { FeatureCollection, Point } from 'geojson';

export const arrivalFixture: FeatureCollection<Point> = {
  type: 'FeatureCollection',
  features: [
    { name: 'Reachable', arrivalMarginMeters: 250.4, arrivalStatus: 'reachable' },
    { name: 'Below reserve', arrivalMarginMeters: -100, arrivalStatus: 'belowReserve' },
    { name: 'Unreachable', arrivalMarginMeters: -200, arrivalStatus: 'unreachable' },
    { name: 'Stale', arrivalMarginMeters: 250, arrivalStatus: 'reachable', arrivalStale: true },
    { name: 'Unavailable' },
    { name: 'At reserve', arrivalMarginMeters: 0, arrivalStatus: 'reachable' },
  ].map((properties, index) => ({
    type: 'Feature',
    id: `1:0:${index}`,
    geometry: {
      type: 'Point',
      coordinates: [6.156 + (index % 3) * 0.03, 50.833 - Math.floor(index / 3) * 0.02],
    },
    properties: { catalogGeneration: 1, kind: 2, runwayDirection: 90, ...properties },
  })),
};
