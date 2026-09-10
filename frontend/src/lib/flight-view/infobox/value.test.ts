import { describe, expect, it } from 'vitest';

import { formatInfoboxValue } from './value';

describe('formatInfoboxValue', () => {
  it('formats speed, signed climb, compass direction and zoom', () => {
    expect(formatInfoboxValue({ kind: 'speed', metersPerSecond: 30, unit: 'km/h' }, 'en')).toEqual({
      text: '108',
      unit: 'km/h',
    });
    expect(
      formatInfoboxValue({ kind: 'vertical-speed', metersPerSecond: 1.25, unit: 'm/s' }, 'en'),
    ).toEqual({ text: '+1.3', unit: 'm/s' });
    expect(
      formatInfoboxValue({ kind: 'vertical-speed', metersPerSecond: -1, unit: 'ft/min' }, 'en'),
    ).toEqual({ text: '-197', unit: 'ft/min' });
    expect(formatInfoboxValue({ kind: 'direction', degrees: 359.8 }, 'en')).toEqual({
      text: '0',
      unit: '°',
    });
    expect(formatInfoboxValue({ kind: 'zoom', level: 11.126 }, 'de')).toEqual({ text: '11,13' });
  });

  it.each([
    [-12.5, '13', -1],
    [12.5, '13', 1],
    [-0.4, '0', 0],
    [0, '0', 0],
    [0.4, '0', 0],
  ])('formats relative angle %s without a sign', (degrees, text, direction) => {
    expect(formatInfoboxValue({ kind: 'relative-angle', degrees }, 'en')).toEqual({
      text,
      unit: '°',
      direction,
    });
  });

  it('shows missing values without a unit or direction', () => {
    expect(formatInfoboxValue({ kind: 'relative-angle', degrees: null }, 'en')).toEqual({
      text: '–',
    });
    expect(formatInfoboxValue({ kind: 'altitude', meters: null, unit: 'ft' }, 'en')).toEqual({
      text: '–',
    });
  });
});
