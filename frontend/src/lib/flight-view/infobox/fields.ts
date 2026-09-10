import type { Instruments } from '$lib/protocol/generated/Instruments';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
import type { InfoboxValue } from './value';

import { m } from '$lib/paraglide/messages';

export type InfoboxField = {
  id: string;
  label: string;
  value: InfoboxValue;
  stale: boolean;
};

export function flightInfoboxes(
  instruments: Instruments,
  units: UnitSettings,
  zoom: number,
): InfoboxField[] {
  let { gps, altitudeAgl, derived } = instruments;
  let { altitude, bank, vario, netto, wind } = derived ?? {};
  let direct = instruments.trueAirspeed;
  let estimated = derived?.airspeed;
  let tas =
    direct && !direct.stale
      ? direct
      : estimated && !estimated.stale
        ? estimated
        : (direct ?? estimated);
  return [
    {
      id: 'altitude',
      label: m.altitude_label(),
      stale: altitude?.stale ?? false,
      value: {
        kind: 'altitude',
        meters: altitude?.altitudeMslMeters ?? null,
        unit: units.altitude,
      },
    },
    {
      id: 'agl',
      label: m.infobox_agl(),
      stale: altitudeAgl?.stale ?? false,
      value: { kind: 'altitude', meters: altitudeAgl?.meters ?? null, unit: units.altitude },
    },
    {
      id: 'ground-speed',
      label: m.infobox_ground_speed(),
      stale: gps?.stale ?? false,
      value: {
        kind: 'speed',
        metersPerSecond: gps?.groundSpeedMetersPerSecond ?? null,
        unit: units.speed,
      },
    },
    {
      id: 'tas',
      label: m.infobox_tas(),
      stale: tas?.stale ?? false,
      value: { kind: 'speed', metersPerSecond: tas?.metersPerSecond ?? null, unit: units.speed },
    },
    {
      id: 'bank',
      label: m.infobox_bank(),
      stale: bank?.stale ?? false,
      value: { kind: 'relative-angle', degrees: bank?.angleDegrees ?? null },
    },
    {
      id: 'vario',
      label: m.infobox_vario(),
      stale: vario?.stale ?? false,
      value: {
        kind: 'vertical-speed',
        metersPerSecond: vario?.metersPerSecond ?? null,
        unit: units.verticalSpeed,
      },
    },
    {
      id: 'netto',
      label: m.infobox_netto(),
      stale: netto?.stale ?? false,
      value: {
        kind: 'vertical-speed',
        metersPerSecond: netto?.metersPerSecond ?? null,
        unit: units.verticalSpeed,
      },
    },
    {
      id: 'wind-speed',
      label: m.infobox_wind_speed(),
      stale: wind?.stale ?? false,
      value: {
        kind: 'speed',
        metersPerSecond: wind?.speedMetersPerSecond ?? null,
        unit: units.speed,
      },
    },
    {
      id: 'wind-direction',
      label: m.infobox_wind_direction(),
      stale: wind?.stale ?? false,
      value: { kind: 'direction', degrees: wind?.directionDegrees ?? null },
    },
    { id: 'zoom', label: m.infobox_zoom(), stale: false, value: { kind: 'zoom', level: zoom } },
  ];
}
