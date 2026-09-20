import type { AppContext } from '$lib/app-context';
import type { FakeClient } from '$lib/client/fake';

import { expect, test } from '@playwright/test';

type TestWindow = Window & {
  __updraftApp?: AppContext;
  __updraftFake?: FakeClient;
};

test('terrain activation and removal retain the other installed source', async ({ page }) => {
  let errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => (window as TestWindow).__updraftFake);
  await page.evaluate(() => {
    (window as TestWindow).__updraftFake!.emitTerrain({
      generation: 0,
      sources: [
        { sourceName: 'local.terrain', type: 'active' },
        { sourceName: 'remaining.terrain', type: 'active' },
      ],
    });
  });
  async function status() {
    return page.evaluate(() => (window as TestWindow).__updraftApp!.terrain.current);
  }
  await page.getByRole('link', { name: 'Settings', exact: true }).click();
  await page.getByRole('link', { name: 'Data', exact: true }).click();
  await page.getByRole('button', { name: /^local / }).click();
  let toggle = page.getByRole('switch', { name: 'Enabled' });
  await toggle.click();
  await expect(toggle).not.toBeChecked();
  await expect.poll(status).toEqual({
    generation: 1,
    sources: [
      { sourceName: 'local.terrain', type: 'disabled' },
      { sourceName: 'remaining.terrain', type: 'active' },
    ],
  });
  await toggle.click();
  await expect(toggle).toBeChecked();
  await expect.poll(status).toEqual({
    generation: 2,
    sources: [
      { sourceName: 'local.terrain', type: 'active' },
      { sourceName: 'remaining.terrain', type: 'active' },
    ],
  });
  await page.getByRole('button', { name: 'Remove from device' }).click();
  await page.getByRole('button', { name: 'Remove', exact: true }).click();
  await expect(page.getByRole('alertdialog')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /^local / })).toHaveCount(0);
  await expect(page.getByRole('button', { name: /^remaining / })).toBeVisible();
  await expect
    .poll(status)
    .toEqual({ generation: 3, sources: [{ sourceName: 'remaining.terrain', type: 'active' }] });
  expect(errors).toEqual([]);
});
