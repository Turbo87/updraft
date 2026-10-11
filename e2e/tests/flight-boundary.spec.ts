import { expect } from '@playwright/test';

import { test } from './app';

test('asks about a position jump and closes the question after New flight', async ({
  page,
  app,
}) => {
  await app.open('/');
  await app.emit({
    topic: 'flightBoundaryQuestion',
    value: { cause: 'positionJump', meters: 120_400 },
  });

  let dialog = page.getByRole('alertdialog', { name: 'New flight?' });
  await expect(dialog).toContainText('Position jumped 120 km.');
  await dialog.getByRole('button', { name: 'New flight' }).click();

  await expect(dialog).toBeHidden();
  expect(await app.flightCommands()).toEqual(['startNewFlight']);
});
