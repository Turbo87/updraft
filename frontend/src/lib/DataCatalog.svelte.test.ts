import type { EnrouteCatalogStatus } from './client';

import { afterEach, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import '../app.css';
import 'virtual:uno.css';

import DataCatalog from './DataCatalog.svelte';
import { applyLocaleSetting } from './i18n.svelte';

const catalog: EnrouteCatalogStatus = {
  cached: {
    checkedAt: Date.UTC(2026, 8, 8),
    entries: [
      {
        path: 'Europe/Germany.mbtiles',
        countryCode: 'DE',
        continent: 'europe',
        size: 10,
        publicationDate: '2026-09-08',
      },
      {
        path: 'Europe/France/North.mbtiles',
        countryCode: 'FR',
        continent: 'europe',
        size: 20,
        publicationDate: '2026-09-08',
      },
      {
        path: 'Europe/France/South.mbtiles',
        countryCode: 'FR',
        continent: 'europe',
        size: 30,
        publicationDate: '2026-09-08',
      },
      {
        path: 'Africa/Morocco.mbtiles',
        countryCode: 'MA',
        continent: 'africa',
        size: 40,
        publicationDate: '2026-09-08',
      },
    ],
  },
  refreshing: false,
  error: false,
};

function props(status: EnrouteCatalogStatus | null = catalog) {
  return { status, onCountry: vi.fn(), onImport: vi.fn(), onRetry: vi.fn() };
}

afterEach(() => applyLocaleSetting('en'));

it('groups published countries and combines regional entries into one country row', async () => {
  let options = props();
  await render(DataCatalog, options);
  expect(
    page
      .getByRole('heading', { level: 2 })
      .elements()
      .map((el) => el.textContent),
  ).toEqual(['Africa', 'Europe']);
  expect(document.querySelectorAll('.country')).toHaveLength(3);
  expect(
    Array.from(document.querySelectorAll('.country')).map((el) => el.textContent?.trim()),
  ).toEqual(['Morocco', 'France', 'Germany']);
  await page.getByRole('button', { name: 'France', exact: true }).click();
  expect(options.onCountry).toHaveBeenCalledExactlyOnceWith('FR');
  let flag = document.querySelector('.i-circle-flags-de')!;
  expect(getComputedStyle(flag).backgroundImage).not.toBe('none');
});

it('updates localized country and continent names when the language changes', async () => {
  await render(DataCatalog, props());
  applyLocaleSetting('de');
  await expect
    .element(page.getByRole('button', { name: 'Deutschland', exact: true }))
    .toBeVisible();
  await expect.element(page.getByRole('heading', { name: 'Europa', exact: true })).toBeVisible();
});

it.each([false, true])(
  'keeps cached countries and import available after failure with refreshing=%s',
  async (refreshing) => {
    let options = props({ ...catalog, error: true, refreshing });
    await render(DataCatalog, options);
    await expect.element(page.getByText('Enroute could not be reached')).toBeVisible();
    await expect.element(page.getByText(/^Last checked /)).toBeVisible();
    await expect.element(page.getByRole('button', { name: 'France', exact: true })).toBeEnabled();
    let retry = page.getByRole('button', { name: 'Retry', exact: true });
    if (refreshing) await expect.element(retry).toBeDisabled();
    else {
      await retry.click();
      expect(options.onRetry).toHaveBeenCalledExactlyOnceWith();
    }
    await page.getByRole('button', { name: 'Import custom file…', exact: true }).click();
    expect(options.onImport).toHaveBeenCalledExactlyOnceWith();
  },
);

it.each([
  null,
  { cached: null, refreshing: true, error: false },
  { cached: null, refreshing: false, error: true },
  { cached: { entries: [], checkedAt: 0 }, refreshing: false, error: false },
])('keeps import available without catalog entries: %j', async (status) => {
  let options = props(status);
  await render(DataCatalog, options);
  expect(document.querySelectorAll('.country')).toHaveLength(0);
  let loading = status === null || status.refreshing;
  await expect
    .element(
      page.getByText(loading ? 'Loading countries…' : 'No downloads available', { exact: true }),
    )
    .toBeVisible();
  await page.getByRole('button', { name: 'Import custom file…', exact: true }).click();
  expect(options.onImport).toHaveBeenCalledExactlyOnceWith();
});

it('replaces the loading state when a cached catalog arrives', async () => {
  let screen = await render(DataCatalog, props(null));
  await expect.element(page.getByText('Loading countries…')).toBeVisible();
  await screen.rerender({ status: catalog });
  await expect.element(page.getByRole('button', { name: 'Germany', exact: true })).toBeVisible();
  await expect.element(page.getByText('Loading countries…')).not.toBeInTheDocument();
});

it.each([413, 915])('keeps country targets and import usable at %ipx', async (width) => {
  let previous = { width: window.innerWidth, height: window.innerHeight };
  await page.viewport(width, 600);
  try {
    let options = props();
    await render(DataCatalog, options);
    let scaffold = document.querySelector<HTMLElement>('.screen-scaffold')!;
    scaffold.style.setProperty('--safe-area-left', '11px');
    scaffold.style.setProperty('--safe-area-right', '23px');
    let country = page.getByRole('button', { name: 'Germany', exact: true });
    let element = country.element() as HTMLButtonElement;
    expect(element.getBoundingClientRect().height).toBeGreaterThanOrEqual(48);
    expect(getComputedStyle(element).paddingLeft).toBe(width === 413 ? '35px' : '24px');
    expect(getComputedStyle(element).paddingRight).toBe(width === 413 ? '43px' : '20px');
    await userEvent.tab();
    element.focus();
    await expect.element(country).toHaveFocus();
    expect(getComputedStyle(element).outlineStyle).toBe('solid');
    await userEvent.keyboard('{Enter}');
    expect(options.onCountry).toHaveBeenCalledExactlyOnceWith('DE');
    await page.getByRole('button', { name: 'Import custom file…', exact: true }).click();
    expect(options.onImport).toHaveBeenCalledExactlyOnceWith();
    let visibleActions = Array.from(document.querySelectorAll('button')).filter(
      (button) => button.textContent?.includes('Import custom file') && button.checkVisibility(),
    );
    expect(visibleActions).toHaveLength(1);
    expect(visibleActions[0].closest(width === 413 ? 'footer' : 'header')).not.toBeNull();
  } finally {
    await page.viewport(previous.width, previous.height);
  }
});
