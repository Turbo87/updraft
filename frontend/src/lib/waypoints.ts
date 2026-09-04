import type { Feature, Point } from 'geojson';

export type WaypointProperties = {
  id: string;
  sourceName: string;
  name: string;
  kind: number;
  elevationMeters: number;
  runwayDirection?: number;
  runwayLengthMeters?: number;
  runwayWidthMeters?: number;
  frequency: string;
  notes: string;
};

export type WaypointFeature = Feature<Point, WaypointProperties>;
