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

test('selects a country update, handles failure, and returns to the library', async ({ page }) => {
  await page.goto('/settings?testMode=1');
  await page.getByRole('link', { name: 'Data', exact: true }).click();
  await page.waitForFunction(() => '__updraftFake' in window);
  await page.evaluate(() => {
    let app = (window as TestWindow).__updraftApp!;
    app.client.selectDataFile = async () => {
      throw new Error('Unexpected file picker');
    };
    let fake = (window as TestWindow).__updraftFake!;
    let path = 'Europe/Malta.mbtiles';
    fake.emitBasemaps({
      generation: 1,
      sources: [{ sourceName: `enroute/${path}`, type: 'disabled' }],
    });
    app.client.getEnrouteBasemapUpdates = async () => [path];
    let attempts = 0;
    app.client.downloadEnrouteFiles = async (paths) => {
      if (paths.length !== 1 || paths[0] !== path) throw new Error('Unexpected download selection');
      if (attempts++ === 0) throw new Error('Submission failed');
      fake.emitEnrouteDownloads([{ path, type: 'queued' }]);
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
  await expect(page.getByRole('main').getByText('Sep 8, 2026', { exact: false })).toBeVisible();
  await page.evaluate(() => history.back());
  await expect(page.getByRole('heading', { name: 'Add data', exact: true })).toBeVisible();
  await page.evaluate(() => history.back());
  await expect(page.getByRole('heading', { name: 'Data', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Add data', exact: true }).click();
  await page.getByRole('button', { name: 'Malta', exact: true }).click();
  let checkbox = page.getByRole('checkbox', { name: 'Malta', exact: true });
  await expect(checkbox).not.toBeChecked();
  await checkbox.check();
  await page.getByRole('button', { name: /^Download/ }).click();
  await expect(page.getByRole('alert')).toHaveText('Could not start the downloads. Try again.');
  await expect(checkbox).toBeChecked();
  await page.getByRole('button', { name: /^Download/ }).click();
  await expect(page.getByRole('heading', { name: 'Data', exact: true })).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Cancel download: Malta', exact: true }),
  ).toBeVisible();
  await expect(page.getByText('Disabled', { exact: true })).toBeVisible();
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

test('keeps download snapshots across settings navigation', async ({ page }) => {
  await page.goto('/settings/data?testMode=1');
  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).__updraftApp?.enrouteDownloads?.current))
    .toEqual([]);
  await page.evaluate(() => {
    (window as TestWindow).__updraftFake!.emitEnrouteDownloads([
      {
        path: 'Europe/Malta.mbtiles',
        type: 'downloading',
        downloaded: 12_000_000,
        total: 100_000_000,
      },
      { path: 'Europe/Germany.mbtiles', type: 'queued' },
    ]);
  });
  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).__updraftApp!.enrouteDownloads.current))
    .toEqual([
      {
        path: 'Europe/Malta.mbtiles',
        type: 'downloading',
        downloaded: 12_000_000,
        total: 100_000_000,
      },
      { path: 'Europe/Germany.mbtiles', type: 'queued' },
    ]);
  await expect(page.getByText('Downloading · 12 MB of 100 MB', { exact: true })).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Cancel download: Germany', exact: true }),
  ).toBeVisible();
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
  await expect(page.getByText('Download failed', { exact: true })).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Retry download: Malta', exact: true }),
  ).toBeVisible();
  await page.evaluate(() => (window as TestWindow).__updraftFake!.emitEnrouteDownloads([]));
  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).__updraftApp!.enrouteDownloads.current))
    .toEqual([]);
});

test('queues France while Germany continues downloading', async ({ page }) => {
  await page.goto('/settings/data?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await page.evaluate(() => {
    let fake = (window as TestWindow).__updraftFake!;
    let client = (window as TestWindow).__updraftApp!.client;
    fake.emitEnrouteCatalog({
      cached: {
        checkedAt: 0,
        entries: ['Germany', 'France'].map((name) => ({
          path: `Europe/${name}.mbtiles`,
          countryCode: name === 'Germany' ? 'DE' : 'FR',
          continent: 'europe',
          size: 100_000_000,
          publicationDate: '2026-09-08',
        })),
      },
      refreshing: false,
      error: false,
    });
    client.downloadEnrouteFiles = async (paths) => {
      if (
        paths.length !== 1 ||
        !['Europe/Germany.mbtiles', 'Europe/France.mbtiles'].includes(paths[0])
      )
        throw new Error('Unexpected download selection');
      fake.emitEnrouteDownloads([
        {
          path: 'Europe/Germany.mbtiles',
          type: 'downloading',
          downloaded: 2_000_000,
          total: 100_000_000,
        },
        ...(paths[0] === 'Europe/France.mbtiles'
          ? [{ path: paths[0], type: 'queued' as const }]
          : []),
      ]);
    };
  });
  await page.getByRole('button', { name: 'Add data', exact: true }).click();
  await page.getByRole('button', { name: 'Germany', exact: true }).click();
  await page.getByRole('checkbox', { name: 'Germany', exact: true }).check();
  await page.getByRole('button', { name: /^Download/ }).click();
  await expect(page.getByRole('heading', { name: 'Data', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Add data', exact: true }).click();
  await page.getByRole('button', { name: 'France', exact: true }).click();
  let checkbox = page.getByRole('checkbox', { name: 'France', exact: true });
  await checkbox.check();
  await page.evaluate(() =>
    (window as TestWindow).__updraftFake!.emitEnrouteDownloads([
      {
        path: 'Europe/Germany.mbtiles',
        type: 'downloading',
        downloaded: 2_000_000,
        total: 100_000_000,
      },
    ]),
  );
  await expect(checkbox).toBeChecked();
  await page.getByRole('button', { name: /^Download/ }).click();
  await expect(page.getByRole('heading', { name: 'Data', exact: true })).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Cancel download: France', exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Cancel download: Germany', exact: true }),
  ).toBeVisible();
});

test('carries startup update results through file replacement and settings navigation', async ({
  page,
}) => {
  await page.goto('/settings?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await page.evaluate(() => {
    let fake = (window as TestWindow).__updraftFake!;
    let path = 'Europe/France.mbtiles';
    fake.getEnrouteBasemapUpdates = async () => [path];
    fake.getBasemapFileDetails = async () => ({ size: 1_000_000, modifiedAt: 0 });
    (window as TestWindow).__updraftApp!.client.downloadEnrouteFiles = async (paths) => {
      if (paths.length !== 1 || paths[0] !== path) throw new Error('Unexpected update selection');
      fake.emitEnrouteDownloads([{ path, type: 'queued' }]);
    };
    fake.emitBasemaps({
      generation: 1,
      sources: [{ sourceName: `enroute/${path}`, type: 'disabled' }],
    });
    fake.emitEnrouteCatalog({
      cached: {
        checkedAt: 1000,
        entries: [
          {
            path,
            countryCode: 'FR',
            continent: 'europe',
            size: 2_000_000,
            publicationDate: '2026-09-08',
          },
        ],
      },
      refreshing: false,
      error: false,
    });
  });
  await page.getByRole('link', { name: 'Data 1 update', exact: true }).click();
  await page.getByRole('button', { name: '1 update available', exact: true }).click();
  await page.getByRole('button', { name: /^France\b/ }).click();
  let dialog = page.getByRole('dialog');
  await expect(dialog.getByText('1 MB', { exact: true })).toBeVisible();
  await expect(dialog.getByRole('switch', { name: 'Enabled', exact: true })).not.toBeChecked();
  await dialog.getByRole('button', { name: 'Update', exact: true }).click();
  await expect(dialog.getByRole('button', { name: 'Update', exact: true })).toBeDisabled();
  await page.evaluate(() => {
    let fake = (window as TestWindow).__updraftFake!;
    fake.getEnrouteBasemapUpdates = async () => [];
    fake.getBasemapFileDetails = async () => ({
      size: 2_000_000,
      modifiedAt: Date.UTC(2026, 8, 8),
    });
    fake.emitBasemaps({
      generation: 2,
      sources: [{ sourceName: 'enroute/Europe/France.mbtiles', type: 'disabled' }],
    });
    fake.emitEnrouteDownloads([]);
  });
  await expect(dialog.getByText('2 MB', { exact: true })).toBeVisible();
  await expect(dialog.getByRole('switch', { name: 'Enabled', exact: true })).not.toBeChecked();
  await expect(dialog.getByRole('button', { name: 'Update', exact: true })).toHaveCount(0);
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(page.getByText('No updates available', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Back to data', exact: true }).click();
  await page.getByRole('link', { name: 'Back to settings', exact: true }).click();
  await expect(page.getByRole('link', { name: 'Data', exact: true })).toBeVisible();
});
