import type { SpeedInstrument } from '$lib/protocol/generated/SpeedInstrument';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

import { describe, expect, it } from 'vitest';

import { EMPTY_DERIVED_INSTRUMENTS, EMPTY_INSTRUMENTS } from '$lib/stores/instruments.svelte';
import { flightInfoboxes } from './fields';

const units: UnitSettings = { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' };
const direct = { metersPerSecond: 30, stale: false };
const derived = { metersPerSecond: 40, stale: false };
const staleDirect = { ...direct, stale: true };
const staleDerived = { ...derived, stale: true };

describe('flightInfoboxes', () => {
  it.each<[SpeedInstrument | null, SpeedInstrument | null, SpeedInstrument | null]>([
    [direct, derived, direct],
    [direct, staleDerived, direct],
    [direct, null, direct],
    [staleDirect, derived, derived],
    [staleDirect, staleDerived, staleDirect],
    [staleDirect, null, staleDirect],
    [null, derived, derived],
    [null, staleDerived, staleDerived],
    [null, null, null],
  ])('selects TAS from direct %j and derived %j', (trueAirspeed, airspeed, expected) => {
    let fields = flightInfoboxes(
      {
        ...EMPTY_INSTRUMENTS,
        trueAirspeed,
        derived: { ...EMPTY_DERIVED_INSTRUMENTS, airspeed },
      },
      units,
    );
    expect(fields.find((field) => field.id === 'tas')).toEqual({
      id: 'tas',
      label: 'TAS',
      stale: expected?.stale ?? false,
      value: { kind: 'speed', metersPerSecond: expected?.metersPerSecond ?? null, unit: 'km/h' },
    });
  });
  it('maps the requested quantities, freshness and units in dock order', () => {
    let fields = flightInfoboxes(
      {
        ...EMPTY_INSTRUMENTS,
        gps: {
          position: { latitudeDegrees: 50, longitudeDegrees: 6 },
          altitudeMeters: 999,
          groundSpeedMetersPerSecond: 31,
          trackDegrees: 90,
          fixTime: null,
          stale: true,
        },
        altitudeAgl: { meters: 780, stale: true },
        derived: {
          ...EMPTY_DERIVED_INSTRUMENTS,
          altitude: { altitudeMslMeters: 1245, stale: false },
          bank: { angleDegrees: -12, stale: false },
          vario: { metersPerSecond: 1.8, stale: true },
          averageVario: { metersPerSecond: 1.4, stale: true },
          netto: { metersPerSecond: 2.5, stale: false },
          wind: { directionDegrees: 248, speedMetersPerSecond: 5, stale: true },
        },
      },
      { altitude: 'ft', speed: 'kt', distance: 'nm', verticalSpeed: 'ft/min' },
    );
    expect(fields.map((field) => field.id)).toEqual([
      'altitude',
      'agl',
      'ground-speed',
      'tas',
      'bank',
      'vario',
      'netto',
      'wind-speed',
      'wind-direction',
      'average-vario',
    ]);
    expect(fields).toMatchInlineSnapshot(`
      [
        {
          "id": "altitude",
          "label": "Altitude",
          "stale": false,
          "value": {
            "kind": "altitude",
            "meters": 1245,
            "unit": "ft",
          },
        },
        {
          "id": "agl",
          "label": "AGL",
          "stale": true,
          "value": {
            "kind": "altitude",
            "meters": 780,
            "unit": "ft",
          },
        },
        {
          "id": "ground-speed",
          "label": "Ground speed",
          "stale": true,
          "value": {
            "kind": "speed",
            "metersPerSecond": 31,
            "unit": "kt",
          },
        },
        {
          "id": "tas",
          "label": "TAS",
          "stale": false,
          "value": {
            "kind": "speed",
            "metersPerSecond": null,
            "unit": "kt",
          },
        },
        {
          "id": "bank",
          "label": "Bank angle",
          "stale": false,
          "value": {
            "degrees": -12,
            "kind": "relative-angle",
          },
        },
        {
          "id": "vario",
          "label": "Vario",
          "stale": true,
          "value": {
            "kind": "vertical-speed",
            "metersPerSecond": 1.8,
            "unit": "ft/min",
          },
        },
        {
          "id": "netto",
          "label": "Netto",
          "stale": false,
          "value": {
            "kind": "vertical-speed",
            "metersPerSecond": 2.5,
            "unit": "ft/min",
          },
        },
        {
          "id": "wind-speed",
          "label": "Wind speed",
          "stale": true,
          "value": {
            "kind": "speed",
            "metersPerSecond": 5,
            "unit": "kt",
          },
        },
        {
          "id": "wind-direction",
          "label": "Wind direction",
          "stale": true,
          "value": {
            "degrees": 248,
            "kind": "direction",
          },
        },
        {
          "id": "average-vario",
          "label": "Avg. vario",
          "stale": true,
          "value": {
            "kind": "vertical-speed",
            "metersPerSecond": 1.4,
            "unit": "ft/min",
          },
        },
      ]
    `);
  });

  it('keeps all slots when instruments are unavailable', () => {
    let fields = flightInfoboxes(EMPTY_INSTRUMENTS, units);
    expect(fields).toHaveLength(10);
    expect(fields).toMatchInlineSnapshot(`
      [
        {
          "id": "altitude",
          "label": "Altitude",
          "stale": false,
          "value": {
            "kind": "altitude",
            "meters": null,
            "unit": "m",
          },
        },
        {
          "id": "agl",
          "label": "AGL",
          "stale": false,
          "value": {
            "kind": "altitude",
            "meters": null,
            "unit": "m",
          },
        },
        {
          "id": "ground-speed",
          "label": "Ground speed",
          "stale": false,
          "value": {
            "kind": "speed",
            "metersPerSecond": null,
            "unit": "km/h",
          },
        },
        {
          "id": "tas",
          "label": "TAS",
          "stale": false,
          "value": {
            "kind": "speed",
            "metersPerSecond": null,
            "unit": "km/h",
          },
        },
        {
          "id": "bank",
          "label": "Bank angle",
          "stale": false,
          "value": {
            "degrees": null,
            "kind": "relative-angle",
          },
        },
        {
          "id": "vario",
          "label": "Vario",
          "stale": false,
          "value": {
            "kind": "vertical-speed",
            "metersPerSecond": null,
            "unit": "m/s",
          },
        },
        {
          "id": "netto",
          "label": "Netto",
          "stale": false,
          "value": {
            "kind": "vertical-speed",
            "metersPerSecond": null,
            "unit": "m/s",
          },
        },
        {
          "id": "wind-speed",
          "label": "Wind speed",
          "stale": false,
          "value": {
            "kind": "speed",
            "metersPerSecond": null,
            "unit": "km/h",
          },
        },
        {
          "id": "wind-direction",
          "label": "Wind direction",
          "stale": false,
          "value": {
            "degrees": null,
            "kind": "direction",
          },
        },
        {
          "id": "average-vario",
          "label": "Avg. vario",
          "stale": false,
          "value": {
            "kind": "vertical-speed",
            "metersPerSecond": null,
            "unit": "m/s",
          },
        },
      ]
    `);
  });
});
