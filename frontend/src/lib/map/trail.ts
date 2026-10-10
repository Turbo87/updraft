import type * as GeoJSON from 'geojson';
import type { ExpressionSpecification, GeoJSONSourceDiff } from 'maplibre-gl';
import type { Trail } from '#lib/protocol/generated/Trail.js';
import type { TrailSample } from '#lib/protocol/generated/TrailSample.js';

import {
  COLOR_INDIGO_700,
  COLOR_RED_600,
  COLOR_SKY_400,
  COLOR_SLATE_400,
  COLOR_YELLOW_400,
} from './colors.generated';

const TRAIL_LENGTH_MILLISECONDS = 60 * 60 * 1000;

type TrailSegmentProperties = { relativeVario: number | null };
type TrailSegment = GeoJSON.Feature<GeoJSON.LineString, TrailSegmentProperties> & { id: number };

/**
 * Sink and lift use different hues. The narrow blend around zero keeps noise
 * in neutral air from striping the trail. `interpolate-lab` clamps values
 * outside the outer stops to the outer colours.
 */
export const TRAIL_COLOR: ExpressionSpecification = [
  'case',
  ['==', ['get', 'relativeVario'], null],
  COLOR_SLATE_400,
  [
    'interpolate-lab',
    ['linear'],
    ['get', 'relativeVario'],
    -5,
    COLOR_INDIGO_700,
    -0.25,
    COLOR_SKY_400,
    0.25,
    COLOR_YELLOW_400,
    5,
    COLOR_RED_600,
  ],
];

/**
 * Keeps the trail segments of the current recording inside the trail length
 * and converts each topic value into one source diff.
 */
export class TrailSegments {
  #recordingStart: number | null = null;
  #previous: TrailSample | null = null;
  #segments: { start: number; feature: TrailSegment }[] = [];

  /** Returns `null` when the source does not change. */
  apply(trail: Trail | null): GeoJSONSourceDiff | null {
    if (trail === null || trail.recordingStart !== this.#recordingStart) {
      let cleared = this.#segments.length > 0;
      this.#recordingStart = trail?.recordingStart ?? null;
      this.#previous = trail?.sample ?? null;
      this.#segments = [];
      return cleared ? { removeAll: true } : null;
    }

    let { sample } = trail;
    let previous = this.#previous;
    this.#previous = sample;
    let since = sample.unixMilliseconds - TRAIL_LENGTH_MILLISECONDS;
    let kept = this.#segments.findIndex(({ start }) => start >= since);
    let removed = this.#segments.splice(0, kept === -1 ? this.#segments.length : kept);
    let added: TrailSegment | null = null;
    if (previous && previous.unixMilliseconds >= since) {
      added = segment(previous, sample);
      this.#segments.push({ start: previous.unixMilliseconds, feature: added });
    }

    if (removed.length === 0 && !added) return null;

    return {
      ...(removed.length > 0 && { remove: removed.map(({ feature }) => feature.id) }),
      ...(added && { add: [added] }),
    };
  }

  featureCollection(): GeoJSON.FeatureCollection<GeoJSON.LineString, TrailSegmentProperties> {
    return { type: 'FeatureCollection', features: this.#segments.map(({ feature }) => feature) };
  }
}

function segment(from: TrailSample, to: TrailSample): TrailSegment {
  return {
    type: 'Feature',
    id: to.unixMilliseconds,
    geometry: {
      type: 'LineString',
      coordinates: [coordinates(from), coordinates(to)],
    },
    properties: { relativeVario: to.relativeVarioMetersPerSecond },
  };
}

function coordinates(sample: TrailSample): GeoJSON.Position {
  return [sample.position.longitudeDegrees, sample.position.latitudeDegrees];
}
