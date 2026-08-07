import { describe, expect, it } from 'vitest';

import { parseNearbyRouteCoordinates } from './params';

describe('parseNearbyRouteCoordinates', () => {
  it.each([
    ['-90', '-180', { latitudeDegrees: -90, longitudeDegrees: -180 }],
    ['0', '0', { latitudeDegrees: 0, longitudeDegrees: 0 }],
    ['90', '180', { latitudeDegrees: 90, longitudeDegrees: 180 }],
  ])('parses latitude %s and longitude %s', (latitude, longitude, expected) => {
    expect(parseNearbyRouteCoordinates(latitude, longitude)).toEqual(expected);
  });

  it.each([
    [undefined, '0'],
    ['0', undefined],
    ['', '0'],
    ['0', ' '],
    ['north', '0'],
    ['0', 'east'],
    ['Infinity', '0'],
    ['0', '-Infinity'],
    ['-90.1', '0'],
    ['90.1', '0'],
    ['0', '-180.1'],
    ['0', '180.1'],
  ])('rejects latitude %s and longitude %s', (latitude, longitude) => {
    expect(parseNearbyRouteCoordinates(latitude, longitude)).toBeNull();
  });
});
