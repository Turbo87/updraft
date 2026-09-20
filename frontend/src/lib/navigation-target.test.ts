import { expect, it } from 'vitest';

import { targetsMatch } from './navigation-target';

it('matches fixed target snapshots by name and coordinate tolerance', () => {
  let target = {
    type: 'waypoint' as const,
    name: 'Home',
    latitudeDegrees: 50,
    longitudeDegrees: 6,
    elevationMeters: 100,
  };
  expect(targetsMatch(target, { ...target, latitudeDegrees: 50.00009, elevationMeters: 200 })).toBe(
    true,
  );
  expect(targetsMatch(target, { ...target, latitudeDegrees: 50.00011 })).toBe(false);
  expect(targetsMatch(target, { ...target, longitudeDegrees: 6.00011 })).toBe(false);
  expect(targetsMatch(target, { ...target, name: 'Alternate' })).toBe(false);
  expect(
    targetsMatch(target, { type: 'mapPosition', latitudeDegrees: 50, longitudeDegrees: 6 }),
  ).toBe(false);
  expect(
    targetsMatch(
      { type: 'mapPosition', latitudeDegrees: 0, longitudeDegrees: -180 },
      { type: 'mapPosition', latitudeDegrees: 0, longitudeDegrees: 180 },
    ),
  ).toBe(true);
});

it('matches traffic by full ID', () => {
  let target = { type: 'traffic' as const, id: 'icao:ABC123' };
  expect(targetsMatch(target, { ...target })).toBe(true);
  expect(targetsMatch(target, { ...target, id: 'flarm:ABC123' })).toBe(false);
});
