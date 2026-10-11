import type { Trail } from '#lib/protocol/generated/Trail';
import type { TestApp } from './app';

import { expect } from '@playwright/test';

import { test } from './app';

test('starts a new flight from the Main Menu after a confirmation', async ({ page, app }) => {
  await app.open('/menu');
  await expect(page.getByRole('heading', { name: 'Main menu', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'New flight' })).toBeHidden();

  await emitRecording(app);
  await page.getByRole('button', { name: 'New flight' }).click();

  let dialog = page.getByRole('alertdialog', { name: 'Start a new flight?' });
  await expect(dialog).toContainText('This deletes the current flight recording.');
  await dialog.getByRole('button', { name: 'New flight' }).click();

  await expect(dialog).toBeHidden();
  expect(await app.flightCommands()).toEqual(['startNewFlight']);
});

test('keeps the confirmation open when the new flight fails', async ({ page, app }) => {
  await app.open('/menu');
  await emitRecording(app);
  await page.evaluate(() => {
    window.__updraftFake!.startNewFlight = () => Promise.reject(new Error('driver stopped'));
  });
  await page.getByRole('button', { name: 'New flight' }).click();

  let dialog = page.getByRole('alertdialog', { name: 'Start a new flight?' });
  await dialog.getByRole('button', { name: 'New flight' }).click();

  await expect(dialog.getByRole('alert')).toHaveText('Could not start a new flight.');
  await expect(dialog).toBeVisible();
});

async function emitRecording(app: TestApp) {
  let value: Trail = {
    recordingStart: 1_767_268_800_000,
    sample: {
      unixMilliseconds: 1_767_268_800_000,
      position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
      altitudeMslMeters: null,
      altitudeAglMeters: null,
      varioMetersPerSecond: null,
      verticalSpeedMetersPerSecond: null,
      nettoMetersPerSecond: null,
      relativeVarioMetersPerSecond: null,
      estimatedNettoMetersPerSecond: null,
      estimatedRelativeVarioMetersPerSecond: null,
    },
    maxAltitudeMslMeters: null,
    maxAltitudeAglMeters: null,
  };
  await app.emit({ topic: 'trail', value });
}
