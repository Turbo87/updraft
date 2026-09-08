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

test('opens the live catalog and country preview without opening the file picker', async ({
  page,
}) => {
  await page.goto('/settings?testMode=1');
  await page.getByRole('link', { name: 'Data', exact: true }).click();
  await page.waitForFunction(() => '__updraftFake' in window);
  await page.evaluate(() => {
    let app = (window as TestWindow).__updraftApp!;
    app.client.selectDataFile = async () => {
      throw new Error('Unexpected file picker');
    };
    (window as TestWindow).__updraftFake!.emitEnrouteCatalog({
      cached: {
        entries: [
          {
            path: 'Europe/Malta.mbtiles',
            countryCode: 'MT',
            continent: 'europe',
            size: 458752,
            publicationDate: '2026-09-08',
          },
        ],
        checkedAt: 0,
      },
      refreshing: false,
      error: false,
    });
  });
  await page.getByRole('button', { name: 'Add data', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Add data', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Malta', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Malta', exact: true })).toBeVisible();
  await expect(page.getByText('Sep 8, 2026', { exact: false })).toBeVisible();
  await page.evaluate(() => history.back());
  await expect(page.getByRole('heading', { name: 'Add data', exact: true })).toBeVisible();
  await page.evaluate(() => history.back());
  await expect(page.getByRole('heading', { name: 'Data', exact: true })).toBeVisible();
});

test('keeps download snapshots across settings navigation', async ({ page }) => {
  await page.goto('/settings/data?testMode=1');
  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).__updraftApp?.enrouteDownloads?.current))
    .toEqual([]);
  await page.evaluate(() => {
    (window as TestWindow).__updraftFake!.emitEnrouteDownloads([
      { path: 'Europe/Malta.mbtiles', type: 'downloading', downloaded: 12, total: 100 },
      { path: 'Europe/Germany.mbtiles', type: 'queued' },
    ]);
  });
  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).__updraftApp!.enrouteDownloads.current))
    .toEqual([
      { path: 'Europe/Malta.mbtiles', type: 'downloading', downloaded: 12, total: 100 },
      { path: 'Europe/Germany.mbtiles', type: 'queued' },
    ]);
  await page.getByRole('link', { name: 'Back to settings', exact: true }).click();
  await page.evaluate(() => {
    (window as TestWindow).__updraftFake!.emitEnrouteDownloads([
      { path: 'Europe/Malta.mbtiles', type: 'failed' },
    ]);
  });
  await page.getByRole('link', { name: 'Data', exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(() => {
        let downloads = (window as TestWindow).__updraftApp!.enrouteDownloads;
        return { current: downloads.current, error: downloads.error };
      }),
    )
    .toEqual({ current: [{ path: 'Europe/Malta.mbtiles', type: 'failed' }], error: false });
  await page.evaluate(() => (window as TestWindow).__updraftFake!.emitEnrouteDownloads([]));
  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).__updraftApp!.enrouteDownloads.current))
    .toEqual([]);
});
