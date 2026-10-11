import type * as GeoJSON from 'geojson';
import type { ExpressionSpecification, GeoJSONSourceDiff } from 'maplibre-gl';
import type { DerivedWindInstruments } from '#lib/protocol/generated/DerivedWindInstruments.js';
import type { Trail } from '#lib/protocol/generated/Trail.js';
import type { TrailColorMode } from '#lib/protocol/generated/TrailColorMode.js';
import type { TrailLength } from '#lib/protocol/generated/TrailLength.js';
import type { TrailSample } from '#lib/protocol/generated/TrailSample.js';

import { preferredValue } from '#lib/preferred-value.js';
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
const EARTH_RADIUS_METERS = 6_371_008.8;
/** The width of the world at zoom 0 in MapLibre pixels. */
const WORLD_PIXELS = 512;
const MAX_ZOOM = 24;

/** A smaller change of the wind estimate keeps the trail, so that each fix does not rebuild it. */
const MIN_WIND_CHANGE_METERS_PER_SECOND = 0.5;

/**
 * `line-translate` moves only the features of the loaded tiles. A rebuild
 * after this age keeps the translation short, so that the trail does not
 * leave its tiles.
 */
const MAX_DRIFT_REFERENCE_AGE_MILLISECONDS = 30 * 1000;

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
type WindowedSegment = { from: TrailSample; to: TrailSample; feature: TrailSegment };

/** The velocity of the air mass. */
type Velocity = { eastMetersPerSecond: number; northMetersPerSecond: number };
/** The segment geometry contains the drift of each point relative to the `reference` UTC. */
type Drift = { wind: Velocity; reference: number };

/** The drift of the whole trail since the reference UTC. */
export type TrailTranslation = { eastMeters: number; northMeters: number; latitudeDegrees: number };

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

/** Returns the `line-translate` of the trail. The pixel distance doubles with each zoom level. */
export function trailTranslate(
  translation: TrailTranslation | null,
): [number, number] | ExpressionSpecification {
  if (!translation) return [0, 0];

  let { eastMeters, northMeters, latitudeDegrees } = translation;
  let circumference = 2 * Math.PI * EARTH_RADIUS_METERS * Math.cos(radians(latitudeDegrees));
  function pixels(zoom: number): ExpressionSpecification {
    let scale = (WORLD_PIXELS * 2 ** zoom) / circumference;
    return ['literal', [eastMeters * scale, -northMeters * scale]];
  }

  return ['interpolate', ['exponential', 2], ['zoom'], 0, pixels(0), MAX_ZOOM, pixels(MAX_ZOOM)];
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
  #drift: Drift | null = null;

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

  /**
   * Moves each point downwind by its age multiplied by `wind`. `null` turns
   * the drift off. Returns `null` when the source does not change.
   */
  drift(wind: DerivedWindInstruments | null): GeoJSONSourceDiff | null {
    let velocity = wind && airMassVelocity(wind);
    let current = this.#drift?.wind ?? null;
    let unchanged = velocity && current ? !changed(current, velocity) : velocity === current;
    if (unchanged) return null;

    this.#drift = velocity && { wind: velocity, reference: 0 };
    let features = this.#rebuild();
    return features.length > 0 ? { removeAll: true, add: features } : null;
  }

  /** Returns `null` without drift. */
  translation(): TrailTranslation | null {
    let newest = this.#previous;
    if (!this.#drift || !newest) return null;

    let seconds = (newest.unixMilliseconds - this.#drift.reference) / 1000;
    return {
      eastMeters: seconds * this.#drift.wind.eastMetersPerSecond,
      northMeters: seconds * this.#drift.wind.northMetersPerSecond,
      latitudeDegrees: newest.position.latitudeDegrees,
    };
  }

  /** Returns `null` when the source does not change. */
  apply(trail: Trail | null): GeoJSONSourceDiff | null {
    if (trail === null || trail.recordingStart !== this.#recordingStart) {
      let cleared = this.#segments.length > 0;
      this.#recordingStart = trail?.recordingStart ?? null;
      this.#previous = trail?.sample ?? null;
      this.#segments = [];
      this.#rebase();
      return cleared ? { removeAll: true } : null;
    }

    let { removed, added } = this.#append(trail.sample);
    let reference = this.#drift?.reference;
    if (
      reference !== undefined &&
      trail.sample.unixMilliseconds - reference > MAX_DRIFT_REFERENCE_AGE_MILLISECONDS
    ) {
      let cleared = removed.length > 0 || this.#segments.length > 0;
      let features = this.#rebuild();
      return cleared ? { removeAll: true, ...(features.length > 0 && { add: features }) } : null;
    }
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
    let sorted = samples.toSorted((a, b) => a.unixMilliseconds - b.unixMilliseconds);
    this.#rebase(sorted.at(-1) ?? null);
    this.#previous = null;
    this.#segments = [];
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
      added = segment(previous, sample, this.#drift);
      this.#segments.push({ from: previous, to: sample, feature: added });
    }
    return { removed, added };
  }

  #trim(since: number): WindowedSegment[] {
    let kept = this.#segments.findIndex(({ from }) => from.unixMilliseconds >= since);
    return this.#segments.splice(0, kept === -1 ? this.#segments.length : kept);
  }

  /** Moves the drift reference to the newest sample, so that the translation is zero. */
  #rebase(newest = this.#previous): void {
    if (this.#drift && newest) this.#drift.reference = newest.unixMilliseconds;
  }

  #rebuild(): TrailSegment[] {
    this.#rebase();
    for (let windowed of this.#segments) {
      windowed.feature = segment(windowed.from, windowed.to, this.#drift);
    }
    return this.#segments.map(({ feature }) => feature);
  }
}

/** The wind direction is the direction that the wind comes from. */
function airMassVelocity({
  directionDegrees,
  speedMetersPerSecond,
}: DerivedWindInstruments): Velocity {
  let direction = radians(directionDegrees);
  return {
    eastMetersPerSecond: -speedMetersPerSecond * Math.sin(direction),
    northMetersPerSecond: -speedMetersPerSecond * Math.cos(direction),
  };
}

function changed(a: Velocity, b: Velocity): boolean {
  let east = a.eastMetersPerSecond - b.eastMetersPerSecond;
  let north = a.northMetersPerSecond - b.northMetersPerSecond;
  return Math.hypot(east, north) > MIN_WIND_CHANGE_METERS_PER_SECOND;
}

function segment(from: TrailSample, to: TrailSample, drift: Drift | null): TrailSegment {
  return {
    type: 'Feature',
    id: to.unixMilliseconds,
    geometry: {
      type: 'LineString',
      coordinates: [coordinates(from, drift), coordinates(to, drift)],
    },
    properties: {
      altitudeMsl: to.altitudeMslMeters,
      altitudeAgl: to.altitudeAglMeters,
      vario: preferredMetersPerSecond(to.varioMetersPerSecond, to.verticalSpeedMetersPerSecond),
      netto: preferredMetersPerSecond(to.nettoMetersPerSecond, to.estimatedNettoMetersPerSecond),
      relativeVario: preferredMetersPerSecond(
        to.relativeVarioMetersPerSecond,
        to.estimatedRelativeVarioMetersPerSecond,
      ),
    },
  };
}

/** A recorded value is null when it was unavailable or stale, so each value is fresh. */
function preferredMetersPerSecond(best: number | null, fallback: number | null): number | null {
  return preferredValue(fresh(best), fresh(fallback))?.value.metersPerSecond ?? null;
}

function fresh(metersPerSecond: number | null): { metersPerSecond: number; stale: false } | null {
  return metersPerSecond === null ? null : { metersPerSecond, stale: false };
}

/** A point older than the drift reference moves downwind, and a newer point moves upwind. */
function coordinates(sample: TrailSample, drift: Drift | null): GeoJSON.Position {
  let { longitudeDegrees, latitudeDegrees } = sample.position;
  if (!drift) return [longitudeDegrees, latitudeDegrees];

  let seconds = (drift.reference - sample.unixMilliseconds) / 1000;
  let east = seconds * drift.wind.eastMetersPerSecond;
  let north = seconds * drift.wind.northMetersPerSecond;
  return [
    longitudeDegrees + degrees(east / (EARTH_RADIUS_METERS * Math.cos(radians(latitudeDegrees)))),
    latitudeDegrees + degrees(north / EARTH_RADIUS_METERS),
  ];
}

function radians(degrees: number): number {
  return (degrees * Math.PI) / 180;
}

function degrees(radians: number): number {
  return (radians * 180) / Math.PI;
}
