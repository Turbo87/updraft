import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import MapSettings from './MapSettings.svelte';

it('selects a hillshade direction and restores the authoritative value after failure', async () => {
  let setDirection = vi.fn().mockRejectedValueOnce(new Error('driver stopped'));
  let screen = await render(MapSettings, { direction: 'fixed', setDirection });
  let fixed = page.getByRole('radio', { name: 'Fixed' });
  let wind = page.getByRole('radio', { name: 'Wind direction' });
  let sun = page.getByRole('radio', { name: 'Sun direction' });
  await expect.element(fixed).toBeChecked();
  await expect.element(sun).not.toBeChecked();

  await wind.click();

  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not change the hillshade direction.');
  await expect.element(fixed).toBeChecked();
  expect(setDirection).toHaveBeenCalledExactlyOnceWith('wind');

  setDirection.mockResolvedValue(undefined);
  await wind.click();
  await screen.rerender({ direction: 'wind' });
  await expect.element(wind).toBeChecked();
  await expect.element(page.getByRole('alert')).not.toBeInTheDocument();
});

it('shows the pending choice until completion and then uses the latest authoritative value', async () => {
  let completion = Promise.withResolvers<void>();
  let screen = await render(MapSettings, {
    direction: 'fixed',
    setDirection: () => completion.promise,
  });
  let wind = page.getByRole('radio', { name: 'Wind direction' });
  let sun = page.getByRole('radio', { name: 'Sun direction' });
  await wind.click();
  await expect.element(wind).toBeChecked();
  await expect.element(wind).toBeDisabled();
  await screen.rerender({ direction: 'sun' });
  await expect.element(wind).toBeChecked();
  completion.resolve();
  await expect.element(sun).toBeChecked();
  await expect.element(wind).toBeEnabled();
});
