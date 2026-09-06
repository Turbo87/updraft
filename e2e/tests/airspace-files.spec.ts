import type { AppContext } from '$lib/app-context';
import type { FakeClient } from '$lib/client/fake';

import { expect, test } from '@playwright/test';

import { AIRSPACE_BROWSER_FIXTURE } from '../../frontend/src/lib/map/airspace.fixture';

type TestWindow = Window & { __updraftApp?: AppContext; __updraftFake?: FakeClient };

test('imports two airspace files, replaces one, and removes only the confirmed file', async ({
  page,
}) => {
  await page.goto('/settings/airspace?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await page.evaluate(() => {
    let client = (window as TestWindow).__updraftFake!;
    let imports = ['a.txt', 'b.txt', 'a.txt'];
    let sources = new Map<string, number>();
    let generation = 0;
    client.importAirspace = async () => {
      let name = imports.shift()!;
      sources.set(name, (sources.get(name) ?? 0) + 1);
      client.emit({
        topic: 'airspace',
        value: {
          generation: ++generation,
          sources: [...sources].map(([sourceName, airspaceCount]) => ({
            type: 'active',
            sourceName,
            airspaceCount,
          })),
        },
      });
      return { type: 'imported' };
    };
  });
  let importButton = page.getByRole('button', { name: 'Import', exact: true });
  await importButton.click();
  await expect(page.getByRole('region', { name: 'a.txt', exact: true })).toBeVisible();
  await importButton.click();
  let first = page.getByRole('region', { name: 'a.txt', exact: true });
  let second = page.getByRole('region', { name: 'b.txt', exact: true });
  await expect(second).toBeVisible();
  await expect(page.getByRole('button', { name: 'Remove airspace source' })).toHaveCount(2);
  await importButton.click();
  await expect(first.getByText('2', { exact: true })).toBeVisible();
  await expect(second.getByText('1', { exact: true })).toBeVisible();
  await first.getByRole('button', { name: 'Remove airspace source' }).click();
  let confirmation = page.getByRole('alertdialog', { name: 'Remove a.txt?' });
  await expect(confirmation).toBeVisible();
  await confirmation.getByRole('button', { name: 'Cancel' }).click();
  await expect(first).toBeVisible();
  await first.getByRole('button', { name: 'Remove airspace source' }).click();
  await confirmation.getByRole('button', { name: 'Remove', exact: true }).click();
  await expect(first).toHaveCount(0);
  await expect(second).toBeVisible();
  await expect(page.getByRole('button', { name: 'Remove airspace source' })).toHaveCount(1);
});

test('keeps duplicate airspaces separate and invalidates details after any source change', async ({
  page,
}) => {
  let feature = AIRSPACE_BROWSER_FIXTURE.features[0];
  let data = {
    type: 'FeatureCollection',
    features: ['a.txt', 'b.txt'].map((sourceName, index) => ({
      ...feature,
      id: `1:${index}:0`,
      properties: { ...feature.properties, id: `1:${index}:0`, sourceName },
    })),
  };
  await page.addInitScript((data) => {
    Object.assign(window, { __updraftTestAirspaceData: data });
  }, data);
  await page.goto('/nearby/50.82/6.175?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await page.evaluate(() => {
    (window as TestWindow).__updraftFake!.emit({
      topic: 'airspace',
      value: {
        generation: 1,
        sources: ['a.txt', 'b.txt'].map((sourceName) => ({
          type: 'active',
          sourceName,
          airspaceCount: 1,
        })),
      },
    });
  });
  let links = page.getByRole('region', { name: 'Airspaces', exact: true }).getByRole('link');
  await expect(links).toHaveCount(2);
  expect(
    (await links.evaluateAll((links) => links.map((link) => link.getAttribute('href')))).sort(),
  ).toEqual(['/airspaces/1:0:0', '/airspaces/1:1:0']);
  await page.locator('a[href="/airspaces/1:1:0"]').click();
  await expect(
    page.getByRole('heading', { level: 1, name: feature.properties.name }),
  ).toBeVisible();
  await page.evaluate(async () => {
    await (window as TestWindow).__updraftFake!.removeAirspace('a.txt');
  });
  await expect(page.getByText('Airspace not found.')).toBeVisible();
});
