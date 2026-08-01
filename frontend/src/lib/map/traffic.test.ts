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
  it('projects a target as a point with complete properties', () => {
    let feature = trafficFeature(target('flarm:000123'));

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
          "altitudeMslMeters": 200,
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
          "altitudeMslMeters": null,
          "stale": false,
          "trackDegrees": null,
          "trafficType": "glider",
        },
        "type": "Feature",
      }
    `);
  });
});

describe('trafficFeatureCollection', () => {
  it('contains one point per target', () => {
    let first = target('flarm:000001');
    let second = target('icao:000002');

    expect(trafficFeatureCollection([first, second]).features.length).toEqual(2);
  });
});

describe('trafficSourceDiff', () => {
  it('adds a new target', () => {
    let added = target('flarm:000001');

    expect(trafficSourceDiff({ upserts: [added], removed: [] })).toEqual({
      add: [trafficFeature(added)],
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

    expect(trafficSourceDiff({ upserts: [updated], removed: [] })).toEqual({
      add: [trafficFeature(updated)],
    });
  });

  it('writes null properties for an updated target without property removal', () => {
    let updated = target('flarm:000001', {
      trackDegrees: null,
      altitudeMslMeters: null,
    });

    let diff = trafficSourceDiff({ upserts: [updated], removed: [] });

    expect(diff).toEqual({
      add: [trafficFeature(updated)],
    });
  });

  it('removes a target without another source operation', () => {
    let removed = 'flarm:000001';

    expect(trafficSourceDiff({ upserts: [], removed: [removed] })).toEqual({
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
    );

    expect(source.setData).toHaveBeenCalledExactlyOnceWith(trafficFeatureCollection([current]));
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
    );

    expect(source.updateData).toHaveBeenCalledExactlyOnceWith(trafficSourceDiff(delta));
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
    );

    expect(warn).toHaveBeenCalledExactlyOnceWith(
      'Traffic source update failed. Rebuilding the source.',
      error,
    );
    expect(source.setData).toHaveBeenCalledExactlyOnceWith(trafficFeatureCollection([updated]));

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
    );

    expect(warn).toHaveBeenCalledExactlyOnceWith(
      'Traffic source update failed. Rebuilding the source.',
      error,
    );
    expect(source.setData).toHaveBeenCalledExactlyOnceWith(trafficFeatureCollection([updated]));
    expect(unsubscribe).toHaveBeenCalledOnce();

    warn.mockRestore();
  });
});
