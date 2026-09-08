import type { ComponentProps } from 'svelte';

import { expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import '../app.css';
import 'virtual:uno.css';

import DataCountry from './DataCountry.svelte';

const north = 'Europe/France/North.mbtiles';
const south = 'Europe/France/South.mbtiles';
function props(): ComponentProps<typeof DataCountry> {
  return {
    country: 'FR',
    entries: [north, south].map((path) => ({
      path,
      countryCode: 'FR',
      continent: 'europe',
      size: 1_000_000,
      publicationDate: '2026-09-08',
    })),
    basemaps: { generation: 0, sources: [] },
    downloads: [],
    client: {
      getEnrouteBasemapUpdates: vi.fn().mockResolvedValue([]),
      downloadEnrouteBasemaps: vi.fn().mockResolvedValue(undefined),
      cancelEnrouteDownload: vi.fn().mockResolvedValue(undefined),
    },
    onBack: vi.fn(),
    onDownloaded: vi.fn(),
  };
}

it.each([1, 2])('starts %i datasets unchecked and waits for queue acceptance', async (count) => {
  let options = props();
  options.entries = options.entries.slice(0, count);
  let accepted = Promise.withResolvers<void>();
  vi.mocked(options.client.downloadEnrouteBasemaps).mockReturnValue(accepted.promise);
  await render(DataCountry, options);
  let checkbox = page.getByRole('checkbox').first();
  await expect.element(checkbox).not.toBeChecked();
  let download = page.getByRole('button', { name: /^Download/ });
  await expect.element(download).toBeDisabled();
  (checkbox.element() as HTMLInputElement).focus();
  await userEvent.keyboard(' ');
  await expect.element(download).toHaveTextContent('Download · 1 MB');
  await download.click();
  expect(options.client.downloadEnrouteBasemaps).toHaveBeenCalledExactlyOnceWith([north]);
  expect(options.onDownloaded).not.toHaveBeenCalled();
  await expect.element(download).toBeDisabled();
  accepted.resolve();
  await vi.waitFor(() => expect(options.onDownloaded).toHaveBeenCalledExactlyOnceWith());
});

it('treats disabled files as installed and allows only detected updates', async () => {
  let options = props();
  options.basemaps = {
    generation: 1,
    sources: [north, south].map((path) => ({ sourceName: `enroute/${path}`, type: 'disabled' })),
  };
  vi.mocked(options.client.getEnrouteBasemapUpdates).mockResolvedValue([south]);
  await render(DataCountry, options);
  await expect.element(page.getByRole('checkbox', { name: 'South', exact: true })).toBeEnabled();
  expect(page.getByRole('checkbox').elements()).toHaveLength(1);
  await expect.element(page.getByText('Installed', { exact: true })).toBeVisible();
  await expect.element(page.getByText('Update available', { exact: true })).toBeVisible();
});

it('rechecks changed inventory, rejects stale results, and retries check failures', async () => {
  let options = props();
  options.basemaps = {
    generation: 1,
    sources: [{ sourceName: `enroute/${north}`, type: 'active' }],
  };
  let first = Promise.withResolvers<string[]>();
  vi.mocked(options.client.getEnrouteBasemapUpdates)
    .mockReturnValueOnce(first.promise)
    .mockRejectedValueOnce(new Error('metadata failed'))
    .mockResolvedValue([]);
  let screen = await render(DataCountry, options);
  await screen.rerender({ basemaps: { ...options.basemaps, generation: 2 } });
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not check for updates. Try again.');
  first.resolve([north]);
  await expect
    .element(page.getByRole('checkbox', { name: 'North', exact: true }))
    .not.toBeInTheDocument();
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect.element(page.getByText('Installed', { exact: true })).toBeVisible();
  await expect.element(page.getByRole('alert')).not.toBeInTheDocument();
});

it('removes live queued selections and cancels without confirmation', async () => {
  let options = props();
  let screen = await render(DataCountry, options);
  await page.getByRole('checkbox', { name: 'North', exact: true }).click();
  await screen.rerender({ downloads: [{ path: north, type: 'queued' }] });
  await expect.element(page.getByRole('button', { name: /^Download/ })).toBeDisabled();
  await expect
    .element(page.getByRole('checkbox', { name: 'North', exact: true }))
    .not.toBeInTheDocument();
  vi.mocked(options.client.cancelEnrouteDownload).mockRejectedValueOnce(new Error('IPC failed'));
  let cancel = page.getByRole('button', { name: 'Cancel download: North', exact: true });
  await cancel.click();
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not cancel the download. Try again.');
  await cancel.click();
  expect(options.client.cancelEnrouteDownload).toHaveBeenCalledTimes(2);
  await screen.rerender({ downloads: [] });
  await expect
    .element(page.getByRole('checkbox', { name: 'North', exact: true }))
    .not.toBeChecked();
});

it('retains selection after submission failure and clears it for another country', async () => {
  let options = props();
  vi.mocked(options.client.downloadEnrouteBasemaps).mockRejectedValueOnce(new Error('IPC failed'));
  let screen = await render(DataCountry, options);
  await page.getByRole('checkbox', { name: 'South', exact: true }).click();
  await page.getByRole('button', { name: /^Download/ }).click();
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not start the downloads. Try again.');
  await expect.element(page.getByRole('checkbox', { name: 'South', exact: true })).toBeChecked();
  expect(options.onDownloaded).not.toHaveBeenCalled();
  await screen.rerender({ country: 'DE', entries: [{ ...options.entries[0], countryCode: 'DE' }] });
  await expect.element(page.getByRole('checkbox')).not.toBeChecked();
});

it('blocks selection while inventory or queue state is unavailable', async () => {
  let screen = await render(DataCountry, { ...props(), downloads: null });
  await expect.element(page.getByRole('checkbox').first()).toBeDisabled();
  await screen.rerender({ downloads: [], basemaps: null, stateError: true });
  await expect
    .element(page.getByRole('alert'))
    .toHaveTextContent('Could not load download state. Restart the app to try again.');
  await expect.element(page.getByRole('checkbox').first()).toBeDisabled();
});

it.each(['close', 'country'])(
  'does not navigate after %s changes during submission',
  async (change) => {
    let options = props();
    let accepted = Promise.withResolvers<void>();
    vi.mocked(options.client.downloadEnrouteBasemaps).mockReturnValue(accepted.promise);
    let screen = await render(DataCountry, options);
    await page.getByRole('checkbox', { name: 'North', exact: true }).click();
    await page.getByRole('button', { name: /^Download/ }).click();
    if (change === 'close') await screen.unmount();
    else await screen.rerender({ country: 'DE' });
    accepted.resolve();
    await accepted.promise;
    expect(options.onDownloaded).not.toHaveBeenCalled();
  },
);

it.each([413, 915])('keeps selection and the Download action usable at %ipx', async (width) => {
  let previous = { width: window.innerWidth, height: window.innerHeight };
  await page.viewport(width, 600);
  try {
    await render(DataCountry, props());
    let checkbox = page.getByRole('checkbox', { name: 'North', exact: true });
    let input = checkbox.element() as HTMLInputElement;
    expect(getComputedStyle(input).appearance).toBe('none');
    expect(input.closest('label')!.getBoundingClientRect().height).toBeGreaterThanOrEqual(48);
    await page.getByText('North', { exact: true }).click();
    await expect.element(checkbox).toBeChecked();
    expect(getComputedStyle(input.nextElementSibling!).visibility).toBe('visible');
    let visibleActions = Array.from(document.querySelectorAll('button')).filter(
      (button) => button.textContent?.includes('Download') && button.checkVisibility(),
    );
    expect(visibleActions).toHaveLength(1);
    expect(visibleActions[0].closest(width === 413 ? 'footer' : 'header')).not.toBeNull();
  } finally {
    await page.viewport(previous.width, previous.height);
  }
});
