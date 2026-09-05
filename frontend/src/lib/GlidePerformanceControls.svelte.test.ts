import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import GlidePerformanceControls from './GlidePerformanceControls.svelte';

it.each([
  ['m/s', '1.5', 1.5],
  ['kt', '1', 1852 / 3600],
  ['ft/min', '100', 0.508],
] as const)('converts MC from %s to meters per second', async (unit, value, expected) => {
  let setMacCready = vi.fn();
  render(GlidePerformanceControls, { macCready: 0, unit, setMacCready });
  let input = page.getByRole('spinbutton', { name: `MC (${unit})` });
  await input.fill(value);
  await page.getByRole('heading').click();
  expect(setMacCready).toHaveBeenCalledExactlyOnceWith(expect.closeTo(expected, 10));
});

it('rejects empty and negative MC and recovers from a failed command', async () => {
  let setMacCready = vi.fn().mockRejectedValue(new Error('driver stopped'));
  render(GlidePerformanceControls, { macCready: 1.5, unit: 'm/s', setMacCready });
  let input = page.getByRole('spinbutton', { name: 'MC (m/s)' });
  for (let value of ['', '-1']) {
    await input.fill(value);
    await page.getByRole('heading').click();
    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Enter a nonnegative MC value.');
  }
  expect(setMacCready).not.toHaveBeenCalled();
  await input.fill('0');
  await page.getByRole('heading').click();
  await expect.element(page.getByRole('alert')).toHaveTextContent('Could not change MC.');
  await expect.element(input).toHaveValue(1.5);
  await expect.element(input).toBeEnabled();
  expect(setMacCready).toHaveBeenCalledExactlyOnceWith(0);
});
