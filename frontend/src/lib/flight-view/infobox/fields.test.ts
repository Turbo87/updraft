import type { SpeedInstrument } from '#lib/protocol/generated/SpeedInstrument.js';
import type { UnitSettings } from '#lib/protocol/generated/UnitSettings.js';

import { describe, expect, it } from 'vitest';

import { m } from '#lib/paraglide/messages.js';
import { EMPTY_DERIVED_INSTRUMENTS, EMPTY_INSTRUMENTS } from '#lib/stores/instruments.svelte.js';
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

  it.each<[SpeedInstrument | null, SpeedInstrument | null, string, SpeedInstrument | null]>([
    [direct, derived, 'Vario', direct],
    [staleDirect, derived, 'Vario (unc.)', derived],
    [null, staleDerived, 'Vario (unc.)', staleDerived],
    [null, null, 'Vario', null],
  ])('shows vario %j or vertical speed %j as %s', (vario, verticalSpeed, label, expected) => {
    let fields = flightInfoboxes(
      { ...EMPTY_INSTRUMENTS, derived: { ...EMPTY_DERIVED_INSTRUMENTS, vario, verticalSpeed } },
      units,
    );
    expect(fields.find((field) => field.id === 'vario')).toEqual({
      id: 'vario',
      label,
      stale: expected?.stale ?? false,
      value: {
        kind: 'vertical-speed',
        metersPerSecond: expected?.metersPerSecond ?? null,
        unit: 'm/s',
      },
    });
  });

  it('labels the vertical speed as an uncompensated vario in German', () => {
    expect(m.infobox_vario_uncompensated({}, { locale: 'de' })).toBe('Vario (unk.)');
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
