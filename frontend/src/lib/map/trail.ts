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
type WindowedSegment = { start: number; feature: TrailSegment };

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

/** Returns the UTC of the oldest sample inside the trail length before `newest`. */
export function trailSince(newest: TrailSample): number {
  return newest.unixMilliseconds - TRAIL_LENGTH_MILLISECONDS;
}

/**
 * Keeps the trail segments of the current recording inside the trail length
 * and converts each change into one source diff.
 */
export class TrailSegments {
  #recordingStart: number | null = null;
  #previous: TrailSample | null = null;
  #segments: WindowedSegment[] = [];

  get recordingStart(): number | null {
    return this.#recordingStart;
  }

  /** Returns `null` when the source does not change. */
  apply(trail: Trail | null): GeoJSONSourceDiff | null {
    if (trail === null || trail.recordingStart !== this.#recordingStart) {
      let cleared = this.#segments.length > 0;
      this.#recordingStart = trail?.recordingStart ?? null;
      this.#previous = trail?.sample ?? null;
      this.#segments = [];
      return cleared ? { removeAll: true } : null;
    }

    let { removed, added } = this.#append(trail.sample);
    if (removed.length === 0 && !added) return null;

    return {
      ...(removed.length > 0 && { remove: removed.map(({ feature }) => feature.id) }),
      ...(added && { add: [added] }),
    };
  }

  /**
   * Replaces the trail with the samples of one recording. The samples can be
   * in any order and can contain duplicates. Returns `null` when the source
   * does not change.
   */
  replace(recordingStart: number, samples: TrailSample[]): GeoJSONSourceDiff | null {
    let cleared = this.#segments.length > 0;
    this.#recordingStart = recordingStart;
    this.#previous = null;
    this.#segments = [];
    let sorted = samples.toSorted((a, b) => a.unixMilliseconds - b.unixMilliseconds);
    for (let sample of sorted) this.#append(sample);

    if (!cleared && this.#segments.length === 0) return null;

    let added = this.#segments.map(({ feature }) => feature);
    return { removeAll: true, ...(added.length > 0 && { add: added }) };
  }

  featureCollection(): GeoJSON.FeatureCollection<GeoJSON.LineString, TrailSegmentProperties> {
    return { type: 'FeatureCollection', features: this.#segments.map(({ feature }) => feature) };
  }

  /** Ignores a sample that is not newer than the previous sample. */
  #append(sample: TrailSample): { removed: WindowedSegment[]; added: TrailSegment | null } {
    let previous = this.#previous;
    if (previous && sample.unixMilliseconds <= previous.unixMilliseconds) {
      return { removed: [], added: null };
    }

    this.#previous = sample;
    let since = trailSince(sample);
    let kept = this.#segments.findIndex(({ start }) => start >= since);
    let removed = this.#segments.splice(0, kept === -1 ? this.#segments.length : kept);
    let added: TrailSegment | null = null;
    if (previous && previous.unixMilliseconds >= since) {
      added = segment(previous, sample);
      this.#segments.push({ start: previous.unixMilliseconds, feature: added });
    }
    return { removed, added };
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
