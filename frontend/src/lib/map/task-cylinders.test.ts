import { distance } from '@turf/distance';
import { expect, it } from 'vitest';

import { taskCylinder } from './task-cylinders';

it.each([
  [6, 50],
  [179.999, 70],
  [-179.999, -70],
])('draws a closed 500 m cylinder at %s, %s without a longitude seam', (longitude, latitude) => {
  let ring = taskCylinder([longitude, latitude]);
  expect(ring).toHaveLength(65);
  expect(ring.at(-1)).toEqual(ring[0]);
  for (let point of ring) {
    expect(distance([longitude, latitude], point, { units: 'meters' })).toBeCloseTo(500, 5);
    expect(Math.abs(point[0] - longitude)).toBeLessThan(1);
  }
});
