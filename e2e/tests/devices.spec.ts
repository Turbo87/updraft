import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';

import { expect } from '@playwright/test';

import { test } from './app';

const device: PublishedExternalDevice = {
  deviceId: 1,
  enabled: true,
  type: 'tcp',
  host: '127.0.0.1',
  port: 4353,
};

test('adds an external device through the client', async ({ page, app }) => {
  await app.open('/settings/devices');
  await page.getByRole('link', { name: 'Add external device' }).click();
  await page.getByLabel('Host').fill('127.0.0.1');
  await page.getByLabel('Port').fill('4353');
  await page.getByRole('button', { name: 'Add external device' }).click();

  await expect(page).toHaveURL(/\/settings\/devices$/);
  expect(await app.externalDeviceCommands()).toEqual([
    ['addExternalDevice', { type: 'tcp', host: '127.0.0.1', port: 4353 }],
  ]);
  await app.emit({ topic: 'externalDevices', value: [device] });
  await expect(page.getByText('127.0.0.1:4353')).toBeVisible();
});

test('enables, edits, and deletes an external device through the client', async ({ page, app }) => {
  await app.open('/settings/devices');
  await app.emit({ topic: 'externalDevices', value: [device] });
  let enabled = page.getByRole('switch', { name: 'Enabled' });
  await enabled.click();
  await expect
    .poll(() => app.externalDeviceCommands())
    .toEqual([['setExternalDeviceEnabled', 1, false]]);
  await expect(enabled).toBeChecked();
  await app.emit({ topic: 'externalDevices', value: [{ ...device, enabled: false }] });
  await expect(enabled).not.toBeChecked();

  await page.getByRole('link', { name: 'Edit 127.0.0.1:4353' }).click();
  await page.getByLabel('Port').fill('10110');
  await page.getByRole('button', { name: 'Save changes' }).click();
  await expect(page).toHaveURL(/\/settings\/devices$/);

  await page.getByRole('link', { name: 'Edit 127.0.0.1:4353' }).click();
  await page.getByRole('button', { name: 'Delete external device' }).click();
  await page.getByRole('alertdialog').getByRole('button', { name: 'Delete' }).click();
  await expect(page).toHaveURL(/\/settings\/devices$/);

  expect(await app.externalDeviceCommands()).toEqual([
    ['setExternalDeviceEnabled', 1, false],
    ['editExternalDevice', 1, { type: 'tcp', host: '127.0.0.1', port: 10110 }],
    ['deleteExternalDevice', 1],
  ]);
});
