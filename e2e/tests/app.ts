import type { Page } from '@playwright/test';
import type { GeoJSONSourceSpecification } from 'maplibre-gl';
import type { AppContext } from '#lib/app-context';
import type { BasemapStatus, TerrainStatus } from '#lib/client';
import type {
  DataFileCommand,
  ExternalDeviceCommand,
  FakeClient,
  NavigationCommand,
  SettingsCommand,
} from '#lib/client/fake';
import type { GpsInstruments } from '#lib/protocol/generated/GpsInstruments';
import type { Navigation } from '#lib/protocol/generated/Navigation';
import type { NavigationTarget } from '#lib/protocol/generated/NavigationTarget';
import type { PinnedTarget } from '#lib/protocol/generated/PinnedTarget';
import type { Topic } from '#lib/protocol/generated/Topic';
import type { TrafficUpdate } from '#lib/protocol/generated/TrafficUpdate';

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

/** Builds a navigation snapshot without guidance. */
export function targetNavigation(target: NavigationTarget): Navigation {
  return {
    target,
    position:
      'latitudeDegrees' in target
        ? { latitudeDegrees: target.latitudeDegrees, longitudeDegrees: target.longitudeDegrees }
        : null,
    guidance: null,
    arrival: null,
    traffic: null,
  };
}

export function pinnedTarget(id: number, target: NavigationTarget, primary = false): PinnedTarget {
  return { id, primary, navigation: targetNavigation(target) };
}

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
    async emitNavigation(target: NavigationTarget | null) {
      await this.emit({ topic: 'navigation', value: target && targetNavigation(target) });
    },
    async emitPins(value: PinnedTarget[]) {
      await this.emit({ topic: 'pinnedTargets', value });
    },
    settingsCommands(): Promise<SettingsCommand[]> {
      return page.evaluate(() => window.__updraftFake!.settingsCommands);
    },
    externalDeviceCommands(): Promise<ExternalDeviceCommand[]> {
      return page.evaluate(() => window.__updraftFake!.externalDeviceCommands);
    },
    dataFileCommands(): Promise<DataFileCommand[]> {
      return page.evaluate(() => window.__updraftFake!.dataFileCommands);
    },
    navigationCommands(): Promise<NavigationCommand[]> {
      return page.evaluate(() => window.__updraftFake!.navigationCommands);
    },
    async queueNavigationReplies(...replies: boolean[]) {
      await page.evaluate(
        (replies) => window.__updraftFake!.queueNavigationReplies(...replies),
        replies,
      );
    },
    async emit(topic: Topic) {
      await page.evaluate((topic) => {
        window.__updraftFake!.emit(topic);
      }, topic);
    },
  };
}
