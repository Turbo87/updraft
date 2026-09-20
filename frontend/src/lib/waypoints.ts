import type { Feature, Point } from 'geojson';

export const WAYPOINT_KIND = {
  UNKNOWN: 0,
  AIRFIELD_GRASS: 2,
  OUTLANDING: 3,
  GLIDING_AIRFIELD: 4,
  AIRFIELD_SOLID: 5,
} as const;

export const LANDABLE_WAYPOINT_KINDS: number[] = [
  WAYPOINT_KIND.AIRFIELD_GRASS,
  WAYPOINT_KIND.OUTLANDING,
  WAYPOINT_KIND.GLIDING_AIRFIELD,
  WAYPOINT_KIND.AIRFIELD_SOLID,
];

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
