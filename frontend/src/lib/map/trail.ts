import type * as GeoJSON from 'geojson';
import type { ExpressionSpecification, GeoJSONSourceDiff } from 'maplibre-gl';
import type { Trail } from '#lib/protocol/generated/Trail.js';
import type { TrailColorMode } from '#lib/protocol/generated/TrailColorMode.js';
import type { TrailLength } from '#lib/protocol/generated/TrailLength.js';
import type { TrailSample } from '#lib/protocol/generated/TrailSample.js';

import { defaultSettings } from '#lib/settings.js';
import {
  COLOR_CYAN_600,
  COLOR_FUCHSIA_600,
  COLOR_INDIGO_700,
  COLOR_LIME_400,
  COLOR_RED_600,
  COLOR_SKY_400,
  COLOR_SLATE_400,
  COLOR_VIOLET_800,
  COLOR_YELLOW_400,
} from './colors.generated';

const MINUTE_MILLISECONDS = 60 * 1000;

/** Keeps the stops of an altitude scale in ascending order. */
const MIN_ALTITUDE_SCALE_METERS = 1;

type TrailSegmentProperties = {
  altitudeMsl: number | null;
  altitudeAgl: number | null;
  vario: number | null;
  netto: number | null;
  relativeVario: number | null;
};
type TrailSegment = GeoJSON.Feature<GeoJSON.LineString, TrailSegmentProperties> & { id: number };
type WindowedSegment = { start: number; feature: TrailSegment };

export type TrailMaxima = Pick<Trail, 'maxAltitudeMslMeters' | 'maxAltitudeAglMeters'>;

/**
 * Returns the `line-color` of the trail. Each segment contains the values
 * of every mode, so a mode change does not change the source data.
 */
export function trailColor(
  mode: TrailColorMode,
  maxima: TrailMaxima,
): string | ExpressionSpecification {
  switch (mode) {
    case 'fixed':
      return COLOR_FUCHSIA_600;
    case 'altitude':
      return altitudeColor('altitudeMsl', maxima.maxAltitudeMslMeters);
    case 'agl':
      return altitudeColor('altitudeAgl', maxima.maxAltitudeAglMeters);
    case 'vario':
    case 'netto':
    case 'relativeVario':
      return climbColor(mode);
  }
}

/**
 * Sink and lift use different hues. The narrow blend around zero keeps noise
 * in neutral air from striping the trail. `interpolate-lab` clamps values
 * outside the outer stops to the outer colours.
 */
function climbColor(property: 'vario' | 'netto' | 'relativeVario'): ExpressionSpecification {
  return colorOrNoValue(property, [
    'interpolate-lab',
    ['linear'],
    ['get', property],
    -5,
    COLOR_INDIGO_700,
    -0.25,
    COLOR_SKY_400,
    0.25,
    COLOR_YELLOW_400,
    5,
    COLOR_RED_600,
  ]);
}

/**
 * The scale goes from 0 m to the maximum of the recording, so a new maximum
 * changes the colour of the older segments.
 */
function altitudeColor(
  property: 'altitudeMsl' | 'altitudeAgl',
  maximum: number | null,
): ExpressionSpecification {
  let top = Math.max(maximum ?? 0, MIN_ALTITUDE_SCALE_METERS);
  return colorOrNoValue(property, [
    'interpolate-lab',
    ['linear'],
    ['get', property],
    0,
    COLOR_VIOLET_800,
    top / 2,
    COLOR_CYAN_600,
    top,
    COLOR_LIME_400,
  ]);
}

function colorOrNoValue(
  property: keyof TrailSegmentProperties,
  color: ExpressionSpecification,
): ExpressionSpecification {
  return ['case', ['==', ['get', property], null], COLOR_SLATE_400, color];
}

/**
 * Keeps the trail segments of the current recording inside the trail length
 * and converts each change into one source diff.
 */
export class TrailSegments {
  #recordingStart: number | null = null;
  #previous: TrailSample | null = null;
  #segments: WindowedSegment[] = [];
  #length: TrailLength = defaultSettings().trailLength;

  get recordingStart(): number | null {
    return this.#recordingStart;
  }

  get length(): TrailLength {
    return this.#length;
  }

  get newest(): TrailSample | null {
    return this.#previous;
  }

  /** Returns the UTC of the oldest sample inside the trail length before `newest`. */
  since(newest: TrailSample): number {
    return newest.unixMilliseconds - this.#length * MINUTE_MILLISECONDS;
  }

  /**
   * Removes the segments outside a shorter length. A longer length does not
   * add segments. Returns `null` when the source does not change.
   */
  resize(length: TrailLength): GeoJSONSourceDiff | null {
    this.#length = length;
    let removed = this.#previous ? this.#trim(this.since(this.#previous)) : [];
    return removed.length > 0 ? { remove: removed.map(({ feature }) => feature.id) } : null;
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
    let since = this.since(sample);
    let removed = this.#trim(since);
    let added: TrailSegment | null = null;
    if (previous && previous.unixMilliseconds >= since) {
      added = segment(previous, sample);
      this.#segments.push({ start: previous.unixMilliseconds, feature: added });
    }
    return { removed, added };
  }

  #trim(since: number): WindowedSegment[] {
    let kept = this.#segments.findIndex(({ start }) => start >= since);
    return this.#segments.splice(0, kept === -1 ? this.#segments.length : kept);
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
    properties: {
      altitudeMsl: to.altitudeMslMeters,
      altitudeAgl: to.altitudeAglMeters,
      vario: to.varioMetersPerSecond,
      netto: to.nettoMetersPerSecond,
      relativeVario: to.relativeVarioMetersPerSecond,
    },
  };
}

function coordinates(sample: TrailSample): GeoJSON.Position {
  return [sample.position.longitudeDegrees, sample.position.latitudeDegrees];
}
