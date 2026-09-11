import type { AltitudeUnit, SpeedUnit, VerticalSpeedUnit } from '$lib/units';

import { convertAltitude, convertSpeed, convertVerticalSpeed } from '$lib/units';

export type InfoboxValue =
  | { kind: 'altitude'; meters: number | null; unit: AltitudeUnit }
  | { kind: 'speed'; metersPerSecond: number | null; unit: SpeedUnit }
  | { kind: 'vertical-speed'; metersPerSecond: number | null; unit: VerticalSpeedUnit }
  | { kind: 'direction'; degrees: number | null }
  | { kind: 'relative-angle'; degrees: number | null };

export type InfoboxPresentation = {
  text: string;
  unit?: string;
  direction?: -1 | 0 | 1;
};

export function formatInfoboxValue(value: InfoboxValue, locale: string): InfoboxPresentation {
  let converted = convertValue(value);
  if (converted === null) return { text: '–' };

  let fractionDigits = value.kind === 'vertical-speed' && value.unit !== 'ft/min' ? 1 : 0;
  let text = new Intl.NumberFormat(locale, {
    maximumFractionDigits: fractionDigits,
    minimumFractionDigits: fractionDigits,
    signDisplay: value.kind === 'vertical-speed' ? 'exceptZero' : 'auto',
    useGrouping: false,
  }).format(converted);

  switch (value.kind) {
    case 'relative-angle':
      return {
        text,
        unit: '°',
        direction: converted === 0 ? 0 : (value.degrees ?? 0) < 0 ? -1 : 1,
      };
    case 'direction':
      return { text, unit: '°' };
    default:
      return { text, unit: value.unit };
  }
}

function convertValue(value: InfoboxValue): number | null {
  switch (value.kind) {
    case 'altitude':
      return value.meters === null ? null : convertAltitude(value.meters, value.unit);
    case 'speed':
      return value.metersPerSecond === null
        ? null
        : convertSpeed(value.metersPerSecond, value.unit);
    case 'vertical-speed':
      return value.metersPerSecond === null
        ? null
        : convertVerticalSpeed(value.metersPerSecond, value.unit);
    case 'direction':
      return value.degrees === null ? null : ((Math.round(value.degrees) % 360) + 360) % 360;
    case 'relative-angle':
      return value.degrees === null ? null : Math.round(Math.abs(value.degrees));
  }
}
