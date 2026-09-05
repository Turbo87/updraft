import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import GlidePerformanceControls from './GlidePerformanceControls.svelte';

it.each(['MC (m/s)', 'Bugs (%)'])('keeps %s focused across an arrow-key save', async (name) => {
  let pending = Promise.withResolvers<void>();
  let save = vi.fn().mockReturnValueOnce(pending.promise).mockResolvedValue(undefined);
  let view = await render(GlidePerformanceControls, {
    macCready: 0,
    unit: 'm/s',
    bugs: 0,
    setMacCready: save,
    setBugs: save,
  });
  let input = page.getByRole('spinbutton', { name, exact: true });
  await input.click();
  await userEvent.keyboard('{ArrowUp}');
  expect(save).toHaveBeenCalledExactlyOnceWith(1);
  await expect.element(input).toHaveFocus();
  await view.rerender(name.startsWith('MC') ? { macCready: 1 } : { bugs: 1 });
  pending.resolve();
  await expect.element(input).not.toHaveAttribute('readonly');
  await userEvent.keyboard('{ArrowUp}');
  expect(save).toHaveBeenLastCalledWith(2);
  expect(save).toHaveBeenCalledTimes(2);
  await expect.element(input).toHaveFocus();
  await view.rerender(name.startsWith('MC') ? { macCready: 2 } : { bugs: 2 });
  await userEvent.keyboard('{ArrowDown}');
  expect(save).toHaveBeenLastCalledWith(1);
  expect(save).toHaveBeenCalledTimes(3);
  await expect.element(input).toHaveFocus();
});

it('rejects invalid bugs and restores the current value after a failed command', async () => {
  let setBugs = vi.fn().mockRejectedValue(new Error('driver stopped'));
  render(GlidePerformanceControls, {
    macCready: 0,
    unit: 'm/s',
    setMacCready: vi.fn(),
    bugs: 0,
    setBugs,
  });
  let input = page.getByRole('spinbutton', { name: 'Bugs (%)', exact: true });
  for (let value of ['', '-1', '100']) {
    await input.fill(value);
    await page.getByRole('heading').click();
    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Enter a bugs value from 0% to less than 100%.');
  }
  expect(setBugs).not.toHaveBeenCalled();
  await input.fill('10.5');
  await page.getByRole('heading').click();
  await expect.element(page.getByRole('alert')).toHaveTextContent('Could not change bugs.');
  await expect.element(input).toHaveValue(0);
  await expect.element(input).toBeEnabled();
  expect(setBugs).toHaveBeenCalledExactlyOnceWith(10.5);
});

it.each([
  ['m/s', '1.5', 1.5],
  ['kt', '1', 1852 / 3600],
  ['ft/min', '100', 0.508],
] as const)('converts MC from %s to meters per second', async (unit, value, expected) => {
  let setMacCready = vi.fn();
  render(GlidePerformanceControls, { macCready: 0, unit, setMacCready, bugs: 0, setBugs: vi.fn() });
  let input = page.getByRole('spinbutton', { name: `MC (${unit})` });
  await input.fill(value);
  await page.getByRole('heading').click();
  expect(setMacCready).toHaveBeenCalledExactlyOnceWith(expect.closeTo(expected, 10));
});

it('rejects empty and negative MC and recovers from a failed command', async () => {
  let setMacCready = vi.fn().mockRejectedValue(new Error('driver stopped'));
  render(GlidePerformanceControls, {
    macCready: 1.5,
    unit: 'm/s',
    setMacCready,
    bugs: 0,
    setBugs: vi.fn(),
  });
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
