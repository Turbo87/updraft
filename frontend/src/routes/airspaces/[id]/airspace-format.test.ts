import { describe, expect, it } from 'vitest';

import {
  formatAirspaceActivity,
  formatAirspaceClass,
  formatAirspaceDateTime,
  formatAirspaceDay,
  formatAirspaceLimit,
  formatAirspaceTime,
  formatAirspaceType,
} from './airspace-format';

describe('formatAirspaceLimit', () => {
  it('formats special limits', () => {
    expect(formatAirspaceLimit({ referenceDatum: 0, unit: 0, value: 0 }, 'ft')).toBe('GND');
    expect(formatAirspaceLimit({ referenceDatum: 2, unit: 6, value: 120 }, 'ft')).toBe('FL 120');
    expect(formatAirspaceLimit({ unlimited: true }, 'ft')).toBe('UNL');
  });

  it('converts limits to the configured altitude unit', () => {
    let limit = { referenceDatum: 0 as const, unit: 1 as const, value: 500 };

    expect(formatAirspaceLimit(limit, 'ft')).toBe('500 ft AGL');
    expect(formatAirspaceLimit(limit, 'm')).toBe('152 m AGL');
    expect(formatAirspaceLimit({ ...limit, referenceDatum: 1 }, 'm')).toBe('152 m MSL');
  });
});

it('formats airspace dates for the selected locale', () => {
  expect(formatAirspaceDateTime('2026-04-12T08:30:00Z', 'en')).toBe('Apr 12, 2026, 8:30 AM');
  expect(formatAirspaceDateTime('2026-04-12T08:30:00Z', 'de')).toBe('12.04.2026, 08:30');
});

it('omits zero seconds from operating times', () => {
  expect(formatAirspaceTime('08:30:00')).toBe('08:30');
  expect(formatAirspaceTime('08:30:15')).toBe('08:30:15');
});

it('formats classifications and operating days for the selected locale', () => {
  expect(formatAirspaceType(4, 'en')).toBe('Control zone');
  expect(formatAirspaceType(4, 'de')).toBe('Kontrollzone');
  expect(formatAirspaceClass(3, 'en')).toBe('Class D');
  expect(formatAirspaceClass(8, 'de')).toBe('Nicht klassifiziert');
  expect(formatAirspaceActivity(5, 'en')).toBe('Hang gliding or paragliding');
  expect(formatAirspaceDay(6, 'de')).toBe('Sonntag');
});
