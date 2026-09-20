import type { Page } from '@playwright/test';
import type { GeoJSONSourceSpecification } from 'maplibre-gl';
import type { AppContext } from '$lib/app-context';
import type { BasemapStatus, TerrainStatus } from '$lib/client';
import type { FakeClient } from '$lib/client/fake';
import type { GpsInstruments } from '$lib/protocol/generated/GpsInstruments';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { TrafficUpdate } from '$lib/protocol/generated/TrafficUpdate';

import { test as base } from '@playwright/test';

declare global {
  interface Window {
    __updraftApp?: AppContext;
    __updraftFake?: FakeClient;
    __updraftTestAirspaceData?: GeoJSONSourceSpecification['data'];
    __updraftTestWaypointData?: GeoJSONSourceSpecification['data'];
  }
}

export const test = base.extend<{ app: TestApp }>({
  app: async ({ page }, use) => {
    await use(createApp(page));
  },
});

export type TestApp = ReturnType<typeof createApp>;

function createApp(page: Page) {
  return {
    async open(path = '/') {
      await page.goto(`${path}?testMode=1`);
      await page.waitForFunction(() => window.__updraftFake);
    },
    async emitInstruments(gps: GpsInstruments) {
      await this.emit({
        topic: 'instruments',
        value: {
          gps,
          pressureAltitude: null,
          trueAirspeed: null,
          derived: null,
          terrainElevation: null,
          altitudeAgl: null,
          solarPosition: null,
        },
      });
    },
    async emitTraffic(value: TrafficUpdate) {
      await this.emit({ topic: 'traffic', value });
    },
    async emitBasemaps(value: BasemapStatus) {
      await page.evaluate((value) => window.__updraftFake!.emitBasemaps(value), value);
    },
    async emitTerrain(value: TerrainStatus) {
      await page.evaluate((value) => window.__updraftFake!.emitTerrain(value), value);
    },
    async emit(topic: Topic) {
      await page.evaluate((topic) => {
        window.__updraftFake!.emit(topic);
      }, topic);
    },
  };
}
