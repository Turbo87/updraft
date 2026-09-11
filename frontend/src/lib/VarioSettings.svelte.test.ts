import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import VarioSettings from './VarioSettings.svelte';

it('selects a method and restores the authoritative value after failure', async () => {
  let setMethod = vi.fn().mockRejectedValueOnce(new Error('driver stopped'));
  let screen = await render(VarioSettings, { method: 'normalizedEma', setMethod });
  let ema = page.getByRole('radio', { name: 'Normalized EMA (10 s)' });
  await expect.element(ema).toBeChecked();
  await page.getByRole('radio', { name: '20 s average', exact: true }).click();
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not change the climb method.');
  await expect.element(ema).toBeChecked();
  expect(setMethod).toHaveBeenCalledExactlyOnceWith('average20s');
  setMethod.mockResolvedValue(undefined);
  await page.getByRole('radio', { name: '30 s average' }).click();
  await screen.rerender({ method: 'average30s' });
  await expect.element(page.getByRole('radio', { name: '30 s average' })).toBeChecked();
  await expect.element(page.getByRole('alert')).not.toBeInTheDocument();
  expect(setMethod).toHaveBeenLastCalledWith('average30s');
  await page.getByRole('radio', { name: 'Smoothed 20 s average' }).click();
  await screen.rerender({ method: 'smoothed20s' });
  await expect.element(page.getByRole('radio', { name: 'Smoothed 20 s average' })).toBeChecked();
  expect(setMethod).toHaveBeenLastCalledWith('smoothed20s');
});
