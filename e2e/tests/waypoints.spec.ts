import type { AppContext } from '$lib/app-context';
import type { FakeClient } from '$lib/client/fake';

import { expect, test } from '@playwright/test';

import { waypointsFixture } from '../../frontend/src/lib/map/waypoint.fixture';

type TestWindow = Window & {
  __updraftApp?: AppContext;
  __updraftFake?: FakeClient;
  __updraftTestWaypointData?: typeof waypointsFixture;
};

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
