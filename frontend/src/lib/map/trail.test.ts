import type { Trail } from '#lib/protocol/generated/Trail.js';
import type { TrailSample } from '#lib/protocol/generated/TrailSample.js';

import { describe, expect, it } from 'vitest';

import { COLOR_SLATE_400 } from './colors.generated';
import { TRAIL_COLOR, TrailSegments } from './trail';

const RECORDING_START = 1_767_268_800_000;
const MINUTE = 60_000;

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
      nettoMetersPerSecond: 2,
      relativeVarioMetersPerSecond: 3,
      ...overrides,
    },
  };
}

describe('TrailSegments', () => {
  it('adds no segment for the first sample', () => {
    let segments = new TrailSegments();

    expect(segments.apply(trail(0))).toBeNull();
    expect(segments.featureCollection()).toEqual({ type: 'FeatureCollection', features: [] });
  });

  it('adds one segment with a stable id for each new sample', () => {
    let segments = new TrailSegments();
    segments.apply(trail(0));

    let diff = segments.apply(trail(1, { relativeVarioMetersPerSecond: null }));

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
              "relativeVario": null,
            },
            "type": "Feature",
          },
        ],
      }
    `);
    expect(segments.featureCollection().features).toEqual(diff?.add);
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

  it('clears the trail when no recording exists', () => {
    let segments = new TrailSegments();
    for (let minutes of [0, 1]) segments.apply(trail(minutes));

    expect(segments.apply(null)).toEqual({ removeAll: true });
    expect(segments.featureCollection().features).toEqual([]);
    expect(segments.apply(trail(2))).toBeNull();
  });
});

describe('TRAIL_COLOR', () => {
  it('uses the neutral colour for a missing relative vario', () => {
    expect(TRAIL_COLOR).toEqual([
      'case',
      ['==', ['get', 'relativeVario'], null],
      COLOR_SLATE_400,
      expect.anything(),
    ]);
  });

  it('interpolates relative vario continuously on a fixed scale from -5 to +5 m/s', () => {
    expect(TRAIL_COLOR[3]).toMatchInlineSnapshot(`
      [
        "interpolate-lab",
        [
          "linear",
        ],
        [
          "get",
          "relativeVario",
        ],
        -5,
        "#432dd7",
        -0.25,
        "#00bcff",
        0.25,
        "#fdc700",
        5,
        "#e7000b",
      ]
    `);
  });
});
