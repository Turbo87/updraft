import type { AppContext } from '$lib/app-context';
import type { FakeClient } from '$lib/client/fake';

import { expect, test } from '@playwright/test';

type TestWindow = Window & { __updraftApp?: AppContext; __updraftFake?: FakeClient };

test('keeps live catalog state across settings navigation', async ({ page }) => {
  await page.goto('/settings/data?testMode=1');
  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).__updraftApp?.enrouteCatalog?.current))
    .toEqual({ cached: null, refreshing: false, error: false });
  let cached = {
    entries: [
      {
        path: 'Europe/Malta.mbtiles',
        countryCode: 'MT',
        continent: 'europe' as const,
        size: 458752,
        publicationDate: '2026-09-08',
      },
    ],
    checkedAt: Date.UTC(2026, 8, 8),
  };
  await page.evaluate(
    (cached) =>
      (window as TestWindow).__updraftFake!.emitEnrouteCatalog({
        cached,
        refreshing: true,
        error: false,
      }),
    cached,
  );
  await page.getByRole('link', { name: 'Back to settings', exact: true }).click();
  await page.evaluate(
    (cached) =>
      (window as TestWindow).__updraftFake!.emitEnrouteCatalog({
        cached,
        refreshing: false,
        error: true,
      }),
    cached,
  );
  await page.getByRole('link', { name: 'Data', exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(() => {
        let catalog = (window as TestWindow).__updraftApp!.enrouteCatalog;
        return { current: catalog.current, error: catalog.error };
      }),
    )
    .toEqual({ current: { cached, refreshing: false, error: true }, error: false });
});
