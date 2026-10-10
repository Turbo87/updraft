import type { ComponentProps } from 'svelte';

import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import MapSettings from './MapSettings.svelte';

function props(overrides: Partial<ComponentProps<typeof MapSettings>> = {}) {
  return {
    direction: 'fixed',
    setDirection: vi.fn(),
    trailColorMode: 'relativeVario',
    setTrailColorMode: vi.fn(),
    ...overrides,
  } satisfies ComponentProps<typeof MapSettings>;
}

it('selects a hillshade direction and restores the authoritative value after failure', async () => {
  let setDirection = vi.fn().mockRejectedValueOnce(new Error('driver stopped'));
  let screen = await render(MapSettings, props({ setDirection }));
  let fixed = page.getByRole('radio', { name: 'Fixed', exact: true });
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
  let screen = await render(MapSettings, props({ setDirection: () => completion.promise }));
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

it('selects a trail colour mode and restores the authoritative value after failure', async () => {
  let setTrailColorMode = vi.fn().mockRejectedValueOnce(new Error('driver stopped'));
  let screen = await render(MapSettings, props({ setTrailColorMode }));
  let group = page.getByRole('group', { name: 'Trail colour' });
  let names = ['Fixed colour', 'Altitude', 'AGL', 'Vario', 'Netto', 'Relative vario'];
  for (let name of names)
    await expect.element(group.getByRole('radio', { name, exact: true })).toBeVisible();
  let relativeVario = group.getByRole('radio', { name: 'Relative vario', exact: true });
  let altitude = group.getByRole('radio', { name: 'Altitude', exact: true });
  await expect.element(relativeVario).toBeChecked();

  await altitude.click();

  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not change the trail colour.');
  await expect.element(relativeVario).toBeChecked();
  expect(setTrailColorMode).toHaveBeenCalledExactlyOnceWith('altitude');

  setTrailColorMode.mockResolvedValue(undefined);
  await altitude.click();
  await screen.rerender({ trailColorMode: 'altitude' });
  await expect.element(altitude).toBeChecked();
  await expect.element(page.getByRole('alert')).not.toBeInTheDocument();
});
