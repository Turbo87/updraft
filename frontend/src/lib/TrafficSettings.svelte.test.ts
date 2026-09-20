import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import TrafficSettings from './TrafficSettings.svelte';

it('restores correction after failure and accepts both modes', async () => {
  let setEnabled = vi.fn().mockRejectedValueOnce(new Error('driver stopped'));
  let screen = await render(TrafficSettings, {
    enabled: true,
    setEnabled,
  });
  let enabled = page.getByRole('radio', { name: /^Enabled/ });
  let disabled = page.getByRole('radio', { name: 'Disabled', exact: true });
  await expect.element(enabled).toBeChecked();
  await disabled.click();
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not change FLARM position correction.');
  await expect.element(enabled).toBeChecked();
  expect(setEnabled).toHaveBeenCalledExactlyOnceWith(false);
  setEnabled.mockResolvedValue(undefined);
  await disabled.click();
  await screen.rerender({ enabled: false });
  await expect.element(disabled).toBeChecked();
  await expect.element(page.getByRole('alert')).not.toBeInTheDocument();
  await enabled.click();
  await screen.rerender({ enabled: true });
  await expect.element(enabled).toBeChecked();
  expect(setEnabled).toHaveBeenLastCalledWith(true);
});

it('keeps a pending disabled choice selected until its command settles', async () => {
  let completion = Promise.withResolvers<void>();
  await render(TrafficSettings, { enabled: true, setEnabled: () => completion.promise });
  let enabled = page.getByRole('radio', { name: /^Enabled/ });
  let disabled = page.getByRole('radio', { name: 'Disabled', exact: true });
  await disabled.click();
  await expect.element(disabled).toBeChecked();
  await expect.element(enabled).toBeDisabled();
  completion.resolve();
  await expect.element(enabled).toBeChecked();
  await expect.element(disabled).toBeEnabled();
});
