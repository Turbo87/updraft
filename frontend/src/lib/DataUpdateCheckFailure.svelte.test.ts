import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import DataUpdateCheckFailure from './DataUpdateCheckFailure.svelte';

it('shows the last successful check and keeps retry available after a failed request', async () => {
  let request = Promise.withResolvers<void>();
  let onRetry = vi.fn(() => request.promise);
  await render(DataUpdateCheckFailure, { checkedAt: 0, onRetry });
  await expect.element(page.getByText(/Last checked/)).toBeVisible();
  await expect.element(page.getByText(/Never checked/)).not.toBeInTheDocument();
  let retry = page.getByRole('button', { name: 'Retry', exact: true });
  await retry.click();
  expect(onRetry).toHaveBeenCalledOnce();
  await expect.element(retry).toBeDisabled();
  request.reject(new Error('offline'));
  await expect.element(page.getByRole('alert')).toHaveTextContent('Could not refresh the catalog');
  await expect.element(retry).toBeEnabled();
  onRetry.mockResolvedValue(undefined);
  await retry.click();
  await expect.element(page.getByRole('alert')).not.toBeInTheDocument();
});

it('shows an unknown check time and disables retry while the catalog refreshes', async () => {
  await render(DataUpdateCheckFailure, { refreshing: true, onRetry: vi.fn() });
  await expect.element(page.getByText('Never checked successfully')).toBeVisible();
  await expect.element(page.getByRole('button', { name: 'Retry', exact: true })).toBeDisabled();
});
