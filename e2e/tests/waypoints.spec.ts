import type { AppContext } from '$lib/app-context';
import type { FakeClient } from '$lib/client/fake';

import { expect, test } from '@playwright/test';

import { waypointsFixture } from '../../frontend/src/lib/map/waypoint.fixture';

type TestWindow = Window & {
  __updraftApp?: AppContext;
  __updraftFake?: FakeClient;
  __updraftTestWaypointData?: typeof waypointsFixture;
};

for (let notes of ['Notes', '']) {
  test(`opens map waypoints and invalidates details after removal (notes: ${notes})`, async ({
    page,
  }) => {
    await page.addInitScript(
      (data) => {
        (window as TestWindow).__updraftTestWaypointData = data;
      },
      {
        ...waypointsFixture,
        features: waypointsFixture.features.map((feature) => ({
          ...feature,
          properties: { ...feature.properties, notes },
        })),
      },
    );
    await page.goto('/?testMode=1');
    await page.waitForFunction(() => '__updraftFake' in window);
    await page.evaluate(() => {
      (window as TestWindow).__updraftFake!.emit({
        topic: 'waypoints',
        value: {
          generation: 1,
          sources: [
            {
              type: 'active',
              sourceName: 'local.cup',
              waypointCount: 3,
              warnings: [],
            },
          ],
        },
      });
    });
    await page.waitForFunction(() => {
      let map = (window as TestWindow).__updraftApp!.mapState.map;
      return map?.getLayer('waypoint-hit') && map.isSourceLoaded('waypoints');
    });
    let point = await page.evaluate(() => {
      let map = (window as TestWindow).__updraftApp!.mapState.map!;
      let point = map.project([6.186, 50.823]);
      let bounds = map.getCanvas().getBoundingClientRect();
      return { x: bounds.x + point.x, y: bounds.y + point.y };
    });
    await page.mouse.click(point.x, point.y);
    let waypoint = page
      .getByRole('region', { name: 'Waypoints', exact: true })
      .getByRole('link', { name: /Point 0/ });
    await expect(waypoint).toContainText(`100 m · 123.500 MHz · ${notes || 'Grass airfield'}`);
    await expect(waypoint).not.toContainText('local.cup');
    await expect(waypoint.locator('.waypoint-symbol')).toBeVisible();
    await expect(waypoint.locator('.runway')).toBeVisible();
    await waypoint.click();
    await expect(page.getByRole('heading', { name: 'Point 0' })).toBeVisible();
    await expect(page.getByText('123.500 MHz')).toBeVisible();
    await expect(page.getByText('090°')).toBeVisible();
    await page.evaluate(async () => {
      await (window as TestWindow).__updraftFake!.removeWaypoints('local.cup');
    });
    await expect(page.getByText('This waypoint is no longer available.')).toBeVisible();
    await page.goBack();
    await expect(page.getByText('No nearby waypoints.')).toBeVisible();
  });
}

test('retries a failed waypoint resource with a new request', async ({ page }) => {
  let available = false;
  let requests = 0;
  await page.route('**/waypoint-resource.geojson', async (route) => {
    requests++;
    await route.fulfill({
      status: available ? 200 : 503,
      contentType: 'application/geo+json',
      body: JSON.stringify(available ? waypointsFixture : {}),
    });
  });
  await page.addInitScript(() => {
    Object.assign(window, { __updraftTestWaypointData: '/waypoint-resource.geojson' });
  });
  await page.goto('/waypoints/1:0:0?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await page.evaluate(() => {
    (window as TestWindow).__updraftFake!.emit({
      topic: 'waypoints',
      value: {
        generation: 1,
        sources: [
          {
            type: 'active',
            sourceName: 'local.cup',
            waypointCount: 3,
            warnings: [],
          },
        ],
      },
    });
  });
  await expect(page.getByText('Could not load waypoints.')).toBeVisible();
  let failedRequests = requests;
  available = true;
  await page.getByRole('button', { name: 'Retry' }).click();
  await expect(page.getByRole('heading', { name: 'Point 0' })).toBeVisible();
  expect(requests).toBeGreaterThan(failedRequests);
});

for (let initialPath of ['/', '/nearby/50.823/6.186']) {
  test(`retains waypoint source errors when opening Nearby from ${initialPath}`, async ({
    page,
  }) => {
    let available = false;
    await page.route('**/waypoint-resource.geojson', async (route) => {
      await route.fulfill({
        status: available ? 200 : 503,
        contentType: 'application/geo+json',
        body: JSON.stringify(available ? waypointsFixture : {}),
      });
    });
    await page.addInitScript(() => {
      Object.assign(window, { __updraftTestWaypointData: '/waypoint-resource.geojson' });
    });
    await page.goto(`${initialPath}?testMode=1`);
    await page.waitForFunction(() => (window as TestWindow).__updraftApp?.mapState.map);
    await page.evaluate(() => {
      let app = (window as TestWindow).__updraftApp!;
      app.mapState.map!.on('error', (event) => {
        if ('sourceId' in event && event.sourceId === 'waypoints')
          document.body.dataset.waypointFailed = 'true';
      });
      (window as TestWindow).__updraftFake!.emit({
        topic: 'waypoints',
        value: {
          generation: 1,
          sources: [
            {
              type: 'active',
              sourceName: 'local.cup',
              waypointCount: 3,
              warnings: [],
            },
          ],
        },
      });
    });
    await expect(page.locator('body')).toHaveAttribute('data-waypoint-failed', 'true');
    if (initialPath === '/') {
      await page.evaluate(() => {
        let link = document.createElement('a');
        link.href = '/nearby/50.823/6.186';
        document.body.append(link);
        link.click();
      });
    }
    let waypoints = page.getByRole('region', { name: 'Waypoints', exact: true });
    await page.evaluate(() => (window as TestWindow).__updraftApp!.mapState.map!.fire('idle'));
    await expect(waypoints.getByRole('alert')).toHaveText('Could not load waypoints.');
    available = true;
    await page.evaluate(async () => {
      let source = (window as TestWindow).__updraftApp!.mapState.map!.getSource(
        'waypoints',
      ) as import('maplibre-gl').GeoJSONSource;
      await source.setData('/waypoint-resource.geojson');
    });
    await expect(waypoints.getByRole('link', { name: /Point 0/ })).toBeVisible();
    await expect(waypoints.getByRole('alert')).not.toBeVisible();
  });
}
