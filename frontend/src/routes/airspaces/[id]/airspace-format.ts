import type { AirspaceLimit } from '$lib/airspace';
import type { AltitudeUnit } from '$lib/protocol/generated/AltitudeUnit';
import type { Locale } from '$lib/protocol/generated/Locale';

import { m } from '$lib/paraglide/messages.js';
import { convertAltitude } from '$lib/units';

export function formatAirspaceLimit(limit: AirspaceLimit, altitudeUnit: AltitudeUnit): string {
  if ('unlimited' in limit) return 'UNL';
  if (limit.unit === 0 && limit.referenceDatum === 0 && limit.value === 0) return 'GND';
  if (limit.unit === 6 && limit.referenceDatum === 2) return `FL ${limit.value}`;

  let meters = limit.unit === 0 ? limit.value : limit.value * 0.3048;
  let value = Math.round(convertAltitude(meters, altitudeUnit));
  let reference = limit.referenceDatum === 0 ? 'AGL' : 'MSL';
  return `${value} ${altitudeUnit} ${reference}`;
}

export function formatAirspaceDateTime(value: string, locale: Locale): string {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: 'medium',
    timeStyle: 'short',
    timeZone: 'UTC',
  }).format(new Date(value));
}

export function formatAirspaceTime(value: string): string {
  return value.endsWith(':00') ? value.slice(0, -3) : value;
}

export function formatAirspaceDay(dayOfWeek: number, locale: Locale): string {
  let date = new Date(Date.UTC(1970, 0, 5 + dayOfWeek));
  return new Intl.DateTimeFormat(locale, { weekday: 'long', timeZone: 'UTC' }).format(date);
}

export function formatAirspaceType(type: number, locale: Locale): string {
  return m.airspace_type_value({ type }, { locale });
}

export function formatAirspaceClass(icaoClass: number, locale: Locale): string {
  return m.icao_class_value({ icaoClass }, { locale });
}

export function formatAirspaceActivity(activity: number, locale: Locale): string {
  return m.airspace_activity_value({ activity }, { locale });
}
