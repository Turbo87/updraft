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
        solarPosition: null,
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
    expect(fields).toMatchSnapshot();
  });

  it('keeps all slots when instruments are unavailable', () => {
    let fields = flightInfoboxes(EMPTY_INSTRUMENTS, units);
    expect(fields).toHaveLength(10);
    expect(fields).toMatchSnapshot();
  });
});
