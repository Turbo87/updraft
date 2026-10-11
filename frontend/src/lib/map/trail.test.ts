import type { Trail } from '#lib/protocol/generated/Trail.js';
import type { TrailSample } from '#lib/protocol/generated/TrailSample.js';

import { describe, expect, it } from 'vitest';

import { COLOR_FUCHSIA_600, COLOR_SLATE_400 } from './colors.generated';
import { trailColor, TrailSegments, trailTranslate } from './trail';

const RECORDING_START = 1_767_268_800_000;
const MINUTE = 60_000;
const MAXIMA = { maxAltitudeMslMeters: 2000, maxAltitudeAglMeters: 1500 };
/** 1 m in degrees of latitude. */
const METER_DEGREES = 180 / (Math.PI * 6_371_008.8);
const NORTH_WIND = { directionDegrees: 0, speedMetersPerSecond: 10, stale: false };

function trail(
  minutes: number,
  overrides: Partial<TrailSample> = {},
  recordingStart = RECORDING_START,
): Trail {
  return {
    recordingStart,
    sample: {
      unixMilliseconds: RECORDING_START + minutes * MINUTE,
      position: { latitudeDegrees: 50 + minutes / 1000, longitudeDegrees: 6 },
      altitudeMslMeters: 1000,
      altitudeAglMeters: 800,
      varioMetersPerSecond: 1,
      verticalSpeedMetersPerSecond: 0.5,
      nettoMetersPerSecond: 2,
      relativeVarioMetersPerSecond: 3,
      estimatedNettoMetersPerSecond: 1.5,
      estimatedRelativeVarioMetersPerSecond: 2.5,
      ...overrides,
    },
    maxAltitudeMslMeters: 1000,
    maxAltitudeAglMeters: 800,
  };
}

describe('TrailSegments', () => {
  it('adds no segment for the first sample', () => {
    let segments = new TrailSegments();

    expect(segments.apply(trail(0))).toBeNull();
    expect(segments.featureCollection()).toEqual({ type: 'FeatureCollection', features: [] });
  });

  it('adds one segment with a stable id and every colour value for each new sample', () => {
    let segments = new TrailSegments();
    segments.apply(trail(0));

    let diff = segments.apply(
      trail(1, { relativeVarioMetersPerSecond: null, estimatedRelativeVarioMetersPerSecond: null }),
    );

    expect(diff).toMatchInlineSnapshot(`
      {
        "add": [
          {
            "geometry": {
              "coordinates": [
                [
                  6,
                  50,
                ],
                [
                  6,
                  50.001,
                ],
              ],
              "type": "LineString",
            },
            "id": 1767268860000,
            "properties": {
              "altitudeAgl": 800,
              "altitudeMsl": 1000,
              "netto": 2,
              "relativeVario": null,
              "vario": 1,
            },
            "type": "Feature",
          },
        ],
      }
    `);
    expect(segments.featureCollection().features).toEqual(diff?.add);
  });

  it('uses the vertical speed as the vario of a sample without a vario', () => {
    let segments = new TrailSegments();
    segments.apply(trail(0));

    let diff = segments.apply(trail(1, { varioMetersPerSecond: null }));

    expect(diff?.add).toMatchObject([{ properties: { vario: 0.5 } }]);
  });

  it('uses the estimated values of a sample without netto and relative vario', () => {
    let segments = new TrailSegments();
    segments.apply(trail(0));

    let diff = segments.apply(
      trail(1, { nettoMetersPerSecond: null, relativeVarioMetersPerSecond: null }),
    );

    expect(diff?.add).toMatchObject([{ properties: { netto: 1.5, relativeVario: 2.5 } }]);
  });

  it('removes the segments that leave 60 min before the newest sample', () => {
    let segments = new TrailSegments();
    for (let minutes of [0, 1, 2]) segments.apply(trail(minutes));

    let diff = segments.apply(trail(61));

    expect(diff?.remove).toEqual([RECORDING_START + MINUTE]);
    expect(diff?.add?.map((feature) => feature.id)).toEqual([RECORDING_START + 61 * MINUTE]);
    expect(segments.featureCollection().features.map((feature) => feature.id)).toEqual([
      RECORDING_START + 2 * MINUTE,
      RECORDING_START + 61 * MINUTE,
    ]);
  });

  it('adds no segment that starts outside the window', () => {
    let segments = new TrailSegments();
    for (let minutes of [0, 1]) segments.apply(trail(minutes));

    let diff = segments.apply(trail(62));

    expect(diff).toEqual({ remove: [RECORDING_START + MINUTE] });
    expect(segments.featureCollection().features).toEqual([]);
    expect(segments.apply(trail(123))).toBeNull();
  });

  it('removes the segments outside a shorter length', () => {
    let segments = new TrailSegments();
    for (let minutes of [0, 1, 2, 3]) segments.apply(trail(minutes));

    expect(segments.resize(1)).toEqual({
      remove: [RECORDING_START + MINUTE, RECORDING_START + 2 * MINUTE],
    });
    expect(segments.featureCollection().features.map(({ id }) => id)).toEqual([
      RECORDING_START + 3 * MINUTE,
    ]);
  });

  it('keeps the segments that a longer length adds to the window', () => {
    let segments = new TrailSegments();
    for (let minutes of [0, 1]) segments.apply(trail(minutes));

    expect(segments.resize(120)).toBeNull();
    expect(segments.apply(trail(100))?.remove).toBeUndefined();
    expect(segments.featureCollection().features).toHaveLength(2);
  });

  it('adds no segment for a length of 0', () => {
    let segments = new TrailSegments();
    segments.resize(0);

    for (let minutes of [0, 1]) expect(segments.apply(trail(minutes))).toBeNull();
    expect(
      segments.replace(
        RECORDING_START,
        [2, 3].map((m) => trail(m).sample),
      ),
    ).toBeNull();
  });

  it('clears the trail when a new recording starts', () => {
    let segments = new TrailSegments();
    for (let minutes of [0, 1]) segments.apply(trail(minutes));

    expect(segments.apply(trail(2, {}, RECORDING_START + 2 * MINUTE))).toEqual({
      removeAll: true,
    });
    expect(segments.featureCollection().features).toEqual([]);
    expect(
      segments.apply(trail(3, {}, RECORDING_START + 2 * MINUTE))?.add?.map(({ id }) => id),
    ).toEqual([RECORDING_START + 3 * MINUTE]);
  });

  it('ignores a sample that is not newer than the previous sample', () => {
    let segments = new TrailSegments();
    for (let minutes of [0, 1]) segments.apply(trail(minutes));

    expect(segments.apply(trail(1))).toBeNull();
    expect(segments.apply(trail(0))).toBeNull();
    expect(segments.featureCollection().features.map(({ id }) => id)).toEqual([
      RECORDING_START + MINUTE,
    ]);
  });

  it('replaces the trail with the samples sorted by UTC without duplicates', () => {
    let segments = new TrailSegments();
    for (let minutes of [0, 1]) segments.apply(trail(minutes, {}, RECORDING_START - MINUTE));
    let samples = [3, 0, 2, 1, 2, 62].map((minutes) => trail(minutes).sample);

    let diff = segments.replace(RECORDING_START, samples);

    expect(diff?.removeAll).toBe(true);
    expect(diff?.add?.map(({ id }) => id)).toEqual([
      RECORDING_START + 3 * MINUTE,
      RECORDING_START + 62 * MINUTE,
    ]);
    expect(segments.recordingStart).toBe(RECORDING_START);
    expect(segments.apply(trail(63))?.add?.map(({ id }) => id)).toEqual([
      RECORDING_START + 63 * MINUTE,
    ]);
  });

  it('builds the segments inside the window from the replaced samples', () => {
    let segments = new TrailSegments();

    let diff = segments.replace(
      RECORDING_START,
      [0, 1, 2].map((minutes) => trail(minutes).sample),
    );

    expect(diff).toEqual({ removeAll: true, add: segments.featureCollection().features });
    expect(diff?.add?.map(({ id }) => id)).toEqual([
      RECORDING_START + MINUTE,
      RECORDING_START + 2 * MINUTE,
    ]);
  });

  it('changes nothing when an empty trail replaces an empty trail', () => {
    let segments = new TrailSegments();

    expect(segments.replace(RECORDING_START, [trail(0).sample])).toBeNull();
    expect(segments.recordingStart).toBe(RECORDING_START);
  });

  it('clears the trail when no recording exists', () => {
    let segments = new TrailSegments();
    for (let minutes of [0, 1]) segments.apply(trail(minutes));

    expect(segments.apply(null)).toEqual({ removeAll: true });
    expect(segments.featureCollection().features).toEqual([]);
    expect(segments.apply(trail(2))).toBeNull();
  });
});

function altitudeColor(property: string, top: number) {
  return [
    'case',
    ['==', ['get', property], null],
    COLOR_SLATE_400,
    [
      'interpolate-lab',
      ['linear'],
      ['get', property],
      0,
      '#5d0ec0',
      top / 2,
      '#0092b8',
      top,
      '#9ae600',
    ],
  ];
}

describe('TrailSegments drift', () => {
  /** Returns a sample of a glider that circles at one position. */
  function circling(seconds: number): Trail {
    return trail(0, { unixMilliseconds: RECORDING_START + seconds * 1000 });
  }

  function coordinates(segments: TrailSegments) {
    return segments.featureCollection().features.map(({ geometry }) => geometry.coordinates);
  }

  function latitudes(segments: TrailSegments) {
    return coordinates(segments).map((line) => line.map(([, latitude]) => latitude));
  }

  it('draws each point upwind by its age before the reference sample', () => {
    let segments = new TrailSegments();
    let westWind = { directionDegrees: 270, speedMetersPerSecond: 10, stale: false };
    let longitudeDegrees = METER_DEGREES / Math.cos((50 * Math.PI) / 180);

    expect(segments.drift(westWind)).toBeNull();
    for (let seconds of [0, 10, 20]) segments.apply(circling(seconds));

    expect(coordinates(segments)).toEqual([
      [
        [6, 50],
        [expect.closeTo(6 - 100 * longitudeDegrees, 12), expect.closeTo(50, 12)],
      ],
      [
        [expect.closeTo(6 - 100 * longitudeDegrees, 12), expect.closeTo(50, 12)],
        [expect.closeTo(6 - 200 * longitudeDegrees, 12), expect.closeTo(50, 12)],
      ],
    ]);
    expect(segments.translation()).toEqual({
      eastMeters: expect.closeTo(200, 9),
      northMeters: expect.closeTo(0, 9),
      latitudeDegrees: 50,
    });
  });

  it('rebuilds the trail from the newest sample after a new wind estimate', () => {
    let segments = new TrailSegments();
    for (let seconds of [0, 10, 20]) segments.apply(circling(seconds));

    let diff = segments.drift(NORTH_WIND);

    expect(diff).toEqual({ removeAll: true, add: segments.featureCollection().features });
    expect(latitudes(segments)).toEqual([
      [expect.closeTo(50 - 200 * METER_DEGREES, 12), expect.closeTo(50 - 100 * METER_DEGREES, 12)],
      [expect.closeTo(50 - 100 * METER_DEGREES, 12), 50],
    ]);
    expect(segments.translation()).toEqual({
      eastMeters: expect.closeTo(0, 9),
      northMeters: expect.closeTo(0, 9),
      latitudeDegrees: 50,
    });
  });

  it('keeps the trail for a wind estimate that changes by 0.5 m/s or less', () => {
    let segments = new TrailSegments();
    for (let seconds of [0, 10]) segments.apply(circling(seconds));
    segments.drift(NORTH_WIND);

    expect(segments.drift({ ...NORTH_WIND, speedMetersPerSecond: 10.5, stale: true })).toBeNull();
    expect(segments.drift({ ...NORTH_WIND, speedMetersPerSecond: 10.6 })?.removeAll).toBe(true);
  });

  it('rebuilds the trail when the reference sample is older than 30 s', () => {
    let segments = new TrailSegments();
    segments.drift(NORTH_WIND);
    for (let seconds of [0, 10, 20]) segments.apply(circling(seconds));

    expect(segments.apply(circling(30))).not.toHaveProperty('removeAll');
    let diff = segments.apply(circling(40));

    expect(diff).toEqual({ removeAll: true, add: segments.featureCollection().features });
    expect(segments.translation()?.northMeters).toBeCloseTo(0, 9);
  });

  it('draws the trail without drift without a wind estimate', () => {
    let segments = new TrailSegments();
    for (let seconds of [0, 10]) segments.apply(circling(seconds));
    segments.drift(NORTH_WIND);

    expect(segments.drift(null)).toEqual({
      removeAll: true,
      add: segments.featureCollection().features,
    });
    expect(coordinates(segments)).toEqual([
      [
        [6, 50],
        [6, 50],
      ],
    ]);
    expect(segments.translation()).toBeNull();
    expect(segments.drift(null)).toBeNull();
  });
});

describe('trailTranslate', () => {
  it('does not move the trail without drift', () => {
    expect(trailTranslate(null)).toEqual([0, 0]);
  });

  it('converts the drift to pixels for each zoom', () => {
    // At 60° latitude, the 512 px world of zoom 0 spans half the equator.
    let pixels = 512 / (Math.PI * 6_371_008.8);
    let drift = { eastMeters: 100, northMeters: -50, latitudeDegrees: 60 };

    expect(trailTranslate(drift)).toEqual([
      'interpolate',
      ['exponential', 2],
      ['zoom'],
      0,
      ['literal', [expect.closeTo(100 * pixels, 12), expect.closeTo(50 * pixels, 12)]],
      24,
      ['literal', [expect.closeTo(100 * pixels * 2 ** 24), expect.closeTo(50 * pixels * 2 ** 24)]],
    ]);
  });
});

describe('trailColor', () => {
  it('uses one colour in the fixed mode', () => {
    expect(trailColor('fixed', MAXIMA)).toBe(COLOR_FUCHSIA_600);
  });

  it.each(['vario', 'netto', 'relativeVario'] as const)(
    'colours %s on a fixed scale from -5 to +5 m/s',
    (mode) => {
      expect(trailColor(mode, MAXIMA)).toEqual([
        'case',
        ['==', ['get', mode], null],
        COLOR_SLATE_400,
        [
          'interpolate-lab',
          ['linear'],
          ['get', mode],
          -5,
          '#432dd7',
          -0.25,
          '#00bcff',
          0.25,
          '#fdc700',
          5,
          '#e7000b',
        ],
      ]);
    },
  );

  it.each([
    ['altitude', 'altitudeMsl', 2000],
    ['agl', 'altitudeAgl', 1500],
  ] as const)('colours %s on a scale from 0 m to the maximum', (mode, property, maximum) => {
    expect(trailColor(mode, MAXIMA)).toEqual(altitudeColor(property, maximum));
  });

  it.each([null, 0, -10])('keeps the altitude scale ascending for a maximum of %s', (maximum) => {
    let maxima = { maxAltitudeMslMeters: maximum, maxAltitudeAglMeters: null };

    expect(trailColor('altitude', maxima)).toEqual(altitudeColor('altitudeMsl', 1));
  });
});
