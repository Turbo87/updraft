import type { Map as MapLibreMap } from 'maplibre-gl';
import type { AirspaceStore } from './stores/airspace.svelte';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import NearbyAirspaces from '../routes/nearby/[latitude]/[longitude]/NearbyAirspaces.svelte';
import NearbyTraffic from '../routes/nearby/[latitude]/[longitude]/NearbyTraffic.svelte';
import NearbyWaypoints from '../routes/nearby/[latitude]/[longitude]/NearbyWaypoints.svelte';
import { TrafficStore } from './stores/traffic.svelte';

const position = { latitudeDegrees: 50.82, longitudeDegrees: 6.18 };

function previewMap(features: unknown[]): MapLibreMap {
  return {
    on() {},
    off() {},
    isStyleLoaded: () => true,
    isSourceLoaded: () => true,
    getSource: () => ({}),
    getLayer: () => ({}),
    project: () => ({ x: 0, y: 0 }),
    queryRenderedFeatures: () => features,
  } as unknown as MapLibreMap;
}

describe('Nearby result cards', () => {
  it.each([
    [413, false],
    [544, false],
    [915, false],
    [413, true],
    [544, true],
    [915, true],
  ])(
    'keeps all Nearby supporting states responsive at %s px (loading: %s)',
    async (width, loading) => {
      let oldWidth = window.innerWidth;
      let oldHeight = window.innerHeight;
      let root = document.documentElement;
      let previousStyle = root.getAttribute('style');
      try {
        await page.viewport(width, 600);
        root.style.setProperty('--safe-area-left', '24px');
        root.style.setProperty('--safe-area-right', '12px');
        let map = previewMap([]);
        map.isStyleLoaded = () => !loading;
        await render(NearbyAirspaces, {
          airspace: { current: { generation: 1, sources: [{ type: 'active' }] } } as AirspaceStore,
          locale: 'en',
          position,
          map,
        });
        await render(NearbyTraffic, {
          locale: 'en',
          position,
          map,
          ownship: null,
          traffic: new TrafficStore(),
          units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
        });
        await render(NearbyWaypoints, {
          altitudeUnit: 'm',
          sourceStatus: loading ? 'loading' : 'ready',
          position,
          map,
        });
        let messages = [...document.querySelectorAll('.empty-results')];
        expect(messages.map((message) => message.textContent)).toEqual(
          loading
            ? ['Loading airspaces…', 'Loading traffic…', 'Loading waypoints…']
            : [
                'No airspace at this position.',
                'No traffic at this position.',
                'No nearby waypoints.',
              ],
        );
        for (let message of messages) {
          expect(getComputedStyle(message.parentElement!).borderRadius).toBe(
            width <= 544 ? '0px' : '12px',
          );
          expect([
            getComputedStyle(message).paddingLeft,
            getComputedStyle(message).paddingRight,
          ]).toEqual(width <= 544 ? ['44px', '32px'] : ['20px', '20px']);
        }
      } finally {
        if (previousStyle === null) root.removeAttribute('style');
        else root.setAttribute('style', previousStyle);
        await page.viewport(oldWidth, oldHeight);
      }
    },
  );

  it.each([413, 915])('uses responsive surfaces around all three lists at %s px', async (width) => {
    let oldWidth = window.innerWidth;
    let oldHeight = window.innerHeight;
    try {
      await page.viewport(width, 600);
      let airspace = { current: { generation: 1, sources: [{ type: 'active' }] } } as AirspaceStore;
      await render(NearbyAirspaces, {
        airspace,
        locale: 'en',
        position,
        map: previewMap([{ id: '1:0', properties: { name: 'Test CTR', type: 4, icaoClass: 3 } }]),
      });
      await render(NearbyWaypoints, {
        altitudeUnit: 'm',
        sourceStatus: 'ready',
        position,
        map: previewMap([
          { properties: { id: 'test:0', name: 'Test airfield', kind: 2, elevationMeters: 60 } },
        ]),
      });
      let traffic = new TrafficStore();
      traffic.apply({
        topic: 'traffic',
        value: {
          type: 'snapshot',
          value: [
            {
              id: 'flarm:ABC123',
              position,
              altitudeMslMeters: 1200,
              trafficType: 'glider',
              trackDegrees: 0,
              alarmLevel: 'none',
              stale: false,
            },
          ],
        },
      });
      await render(NearbyTraffic, {
        locale: 'en',
        position,
        traffic,
        ownship: null,
        units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
        map: previewMap([{ id: 'flarm:ABC123' }]),
      });
      let lists = page.getByRole('list').all();
      expect(lists).toHaveLength(3);
      for (let list of lists) {
        let element = list.element();
        expect(getComputedStyle(element).boxShadow).toBe('none');
        expect(getComputedStyle(element.parentElement!).borderRadius).toBe(
          width < 544 ? '0px' : '12px',
        );
        expect(element.querySelectorAll('a')).toHaveLength(1);
        let link = element.querySelector('a')!;
        link.focus();
        expect(getComputedStyle(link).outlineStyle).toBe('solid');
        expect(getComputedStyle(link).outlineOffset).toBe('-2px');
      }
    } finally {
      await page.viewport(oldWidth, oldHeight);
    }
  });
});
