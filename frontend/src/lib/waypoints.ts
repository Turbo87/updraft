import type { Feature, Point } from 'geojson';

// Grass airfields, outlandings, gliding airfields, and solid airfields.
export const LANDABLE_WAYPOINT_KINDS = [2, 3, 4, 5];

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
