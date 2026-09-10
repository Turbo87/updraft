import type { ErrorEvent } from 'maplibre-gl';
import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';

import { describe, expect, it, vi } from 'vitest';

import {
  applyTrafficSourceUpdate,
  trafficFeature,
  trafficFeatureCollection,
  trafficSourceDiff,
} from './traffic';

function target(
  id: string,
  overrides: Partial<PublishedTrafficTarget> = {},
): PublishedTrafficTarget {
  return {
    id,
    position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
    altitudeMslMeters: 200,
    trafficType: 'glider',
    trackDegrees: 270,
    alarmLevel: 'none',
    stale: false,
    ...overrides,
  };
}

describe('trafficFeature', () => {
  it('projects a target with a whole-meter altitude label', () => {
    let feature = trafficFeature(target('flarm:000123', { altitudeMslMeters: 200.4 }), 'm', 'm/s');

    expect(feature).toMatchInlineSnapshot(`
      {
        "geometry": {
          "coordinates": [
            6.186,
            50.823,
          ],
          "type": "Point",
        },
        "id": "flarm:000123",
        "properties": {
          "alarmLevel": "none",
          "id": "flarm:000123",
          "label": "200 m",
          "stale": false,
          "trackDegrees": 270,
          "trafficType": "glider",
        },
        "type": "Feature",
      }
    `);
  });

  it('projects unknown track and altitude properties as null', () => {
    let feature = trafficFeature(
      target('flarm:000123', { trackDegrees: null, altitudeMslMeters: null }),
      'm',
      'm/s',
    );

    expect(feature).toMatchInlineSnapshot(`
      {
        "geometry": {
          "coordinates": [
            6.186,
            50.823,
          ],
          "type": "Point",
        },
        "id": "flarm:000123",
        "properties": {
          "alarmLevel": "none",
          "id": "flarm:000123",
          "label": null,
          "stale": false,
          "trackDegrees": null,
          "trafficType": "glider",
        },
        "type": "Feature",
      }
    `);
  });

  it('projects a target with a whole-foot altitude label', () => {
    let feature = trafficFeature(target('flarm:000123'), 'ft', 'm/s');

    expect(feature.properties.label).toBe('656 ft');
  });
});

describe('trafficFeatureCollection', () => {
  it('contains one point per target', () => {
    let first = target('flarm:000001');
    let second = target('icao:000002');

    expect(trafficFeatureCollection([first, second], 'm', 'm/s').features.length).toEqual(2);
  });
});

describe('trafficSourceDiff', () => {
  it('adds a new target', () => {
    let added = target('flarm:000001');

    expect(trafficSourceDiff({ upserts: [added], removed: [] }, 'm', 'm/s')).toEqual({
      add: [trafficFeature(added, 'm', 'm/s')],
    });
  });

  it('upserts a complete existing target without source state', () => {
    let updated = target('flarm:000001', {
      position: { latitudeDegrees: 50.824, longitudeDegrees: 6.187 },
      trafficType: 'towPlane',
      alarmLevel: 'important',
      stale: true,
      trackDegrees: 90,
      altitudeMslMeters: 500,
    });

    expect(trafficSourceDiff({ upserts: [updated], removed: [] }, 'm', 'm/s')).toEqual({
      add: [trafficFeature(updated, 'm', 'm/s')],
    });
  });

  it('writes null properties for an updated target without property removal', () => {
    let updated = target('flarm:000001', {
      trackDegrees: null,
      altitudeMslMeters: null,
    });

    let diff = trafficSourceDiff({ upserts: [updated], removed: [] }, 'm', 'm/s');

    expect(diff).toEqual({
      add: [trafficFeature(updated, 'm', 'm/s')],
    });
  });

  it('removes a target without another source operation', () => {
    let removed = 'flarm:000001';

    expect(trafficSourceDiff({ upserts: [], removed: [removed] }, 'm', 'm/s')).toEqual({
      remove: ['flarm:000001'],
    });
  });
});

describe('applyTrafficSourceUpdate', () => {
  it('rebuilds the complete current map for a snapshot', async () => {
    let current = target('flarm:000002');
    let source = {
      setData: vi.fn(async () => {}),
      updateData: vi.fn(async () => {}),
      on: vi.fn(() => ({ unsubscribe: vi.fn() })),
    };

    await applyTrafficSourceUpdate(
      source,
      { type: 'snapshot', value: [current] },
      new Map([['flarm:000002', current]]),
      'm',
      'm/s',
    );

    expect(source.setData).toHaveBeenCalledExactlyOnceWith(
      trafficFeatureCollection([current], 'm', 'm/s'),
    );
    expect(source.updateData).not.toHaveBeenCalled();
  });

  it('applies a delta without reading previous source state', async () => {
    let updated = target('flarm:000001', { trackDegrees: 90 });
    let delta = { upserts: [updated], removed: [] };
    let source = {
      setData: vi.fn(async () => {}),
      updateData: vi.fn(async () => {}),
      on: vi.fn(() => ({ unsubscribe: vi.fn() })),
    };

    await applyTrafficSourceUpdate(
      source,
      { type: 'delta', value: delta },
      new Map([['flarm:000001', updated]]),
      'm',
      'm/s',
    );

    expect(source.updateData).toHaveBeenCalledExactlyOnceWith(trafficSourceDiff(delta, 'm', 'm/s'));
    expect(source.setData).not.toHaveBeenCalled();
  });

  it('warns once and rebuilds the exact current map after a rejected delta', async () => {
    let updated = target('flarm:000001', { trackDegrees: 90 });
    let error = new Error('worker update failed');
    let source = {
      setData: vi.fn(async () => {}),
      updateData: vi.fn(async () => {
        throw error;
      }),
      on: vi.fn(() => ({ unsubscribe: vi.fn() })),
    };
    let warn = vi.spyOn(console, 'warn').mockImplementation(() => {});

    await applyTrafficSourceUpdate(
      source,
      { type: 'delta', value: { upserts: [updated], removed: [] } },
      new Map([['flarm:000001', updated]]),
      'm',
      'm/s',
    );

    expect(warn).toHaveBeenCalledExactlyOnceWith(
      'Traffic source update failed. Rebuilding the source.',
      error,
    );
    expect(source.setData).toHaveBeenCalledExactlyOnceWith(
      trafficFeatureCollection([updated], 'm', 'm/s'),
    );

    warn.mockRestore();
  });

  it('warns once and rebuilds when a resolved delta emits a source error', async () => {
    let updated = target('flarm:000001', { trackDegrees: 90 });
    let error = new Error('worker update failed');
    let errorListener: ((event: ErrorEvent) => void) | undefined;
    let unsubscribe = vi.fn();
    let source = {
      setData: vi.fn(async () => {}),
      updateData: vi.fn(async () => {
        errorListener?.({ error, type: 'error' });
      }),
      on: vi.fn((_type: 'error', listener: (event: ErrorEvent) => void) => {
        errorListener = listener;
        return { unsubscribe };
      }),
    };
    let warn = vi.spyOn(console, 'warn').mockImplementation(() => {});

    await applyTrafficSourceUpdate(
      source,
      { type: 'delta', value: { upserts: [updated], removed: [] } },
      new Map([['flarm:000001', updated]]),
      'm',
      'm/s',
    );

    expect(warn).toHaveBeenCalledExactlyOnceWith(
      'Traffic source update failed. Rebuilding the source.',
      error,
    );
    expect(source.setData).toHaveBeenCalledExactlyOnceWith(
      trafficFeatureCollection([updated], 'm', 'm/s'),
    );
    expect(unsubscribe).toHaveBeenCalledOnce();

    warn.mockRestore();
  });
});

describe('FlarmNet traffic labels', () => {
  it.each([
    ['EL', 'D-TEST', 200, 'EL\n200 m'],
    ['', 'D-TEST', 200, 'D-TEST\n200 m'],
    ['EL', 'D-TEST', null, 'EL'],
    ['', 'D-TEST', null, 'D-TEST'],
    ['', '', 200, '200 m'],
    ['', '', null, null],
  ])(
    'formats callsign %s, registration %s, and altitude %s',
    (callSign, registration, altitudeMslMeters, expected) => {
      let traffic = target('flarm:ABC123', {
        altitudeMslMeters,
        flarmnet: {
          flarmId: 'ABC123',
          callSign,
          registration,
          planeType: '',
          pilotName: '',
          airfield: '',
          frequency: '',
        },
      });
      expect(trafficFeature(traffic, 'm', 'm/s').properties.label).toBe(expected);
    },
  );
});

it('adds positive normalized climb in the selected vertical-speed unit', () => {
  let aircraft = target('flarm:000123', {
    climb: { average20s: 1, average30s: 3, normalizedEma: 2.1 },
  });
  expect(trafficFeature(aircraft, 'm', 'm/s').properties.label).toBe('200 m\n+2.1 m/s');
  expect(trafficFeature(aircraft, 'm', 'kt').properties.label).toBe('200 m\n+4.1 kt');
  expect(trafficFeature(aircraft, 'm', 'ft/min').properties.label).toBe('200 m\n+413 ft/min');
  for (let normalizedEma of [-1, 0, 0.01]) {
    aircraft.climb!.normalizedEma = normalizedEma;
    expect(trafficFeature(aircraft, 'm', 'm/s').properties.label).toBe('200 m');
  }
  aircraft.climb!.normalizedEma = 2;
  aircraft.stale = true;
  expect(trafficFeature(aircraft, 'm', 'm/s').properties.label).toBe('200 m');
});

it('switches the displayed method using estimates already on the target', () => {
  let aircraft = target('flarm:000123', {
    climb: { average20s: 1, average30s: 3, normalizedEma: 2 },
  });
  expect(trafficFeature(aircraft, 'm', 'm/s', 'average20s').properties.label).toBe(
    '200 m\n+1.0 m/s',
  );
  expect(trafficFeature(aircraft, 'm', 'm/s', 'average30s').properties.label).toBe(
    '200 m\n+3.0 m/s',
  );
});
