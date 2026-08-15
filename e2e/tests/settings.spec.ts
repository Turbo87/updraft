import type { AppContext } from '$lib/app-context';

import { execFileSync } from 'node:child_process';

import { expect, test } from '@playwright/test';

const EXPECTED_BUILD_COMMIT_SHA = execFileSync('git', ['rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();

type TestWindow = Window & {
  __airspaceImportCalls?: number;
  __updraftApp?: AppContext;
  __updraftFake?: {
    emit: (topic: unknown) => void;
    importAirspace: () => Promise<{ type: 'cancelled' }>;
  };
};

test.describe('with an unsupported browser language', () => {
  test.use({ locale: 'es-ES' });

  test('falls back to English and changes settings through the backend-shaped fake', async ({
    page,
  }) => {
    await page.goto('/?testMode=1');
    await page.getByRole('link', { name: 'Settings' }).click();
    await page.getByRole('link', { name: 'Language' }).click();

    await expect(page).toHaveURL(/\/settings\/language$/);
    await expect(page.getByRole('radio', { name: 'English' })).toBeChecked();

    await page.getByRole('radio', { name: 'Deutsch' }).click();

    await expect(page.getByRole('heading', { name: 'Sprache' })).toBeVisible();
    await expect(page.getByRole('radio', { name: 'Deutsch' })).toBeChecked();
    await expect(page.locator('html')).toHaveAttribute('lang', 'de');

    await page.getByRole('link', { name: 'Zurück zu den Einstellungen' }).click();
    await expect(page.getByRole('heading', { name: 'Einstellungen' })).toBeVisible();

    await page.getByRole('link', { name: 'Einheiten' }).click();
    let altitude = page.getByRole('combobox', { name: 'Höhe', exact: true });
    await expect(altitude).toHaveValue('m');
    await altitude.selectOption('ft');
    await expect(altitude).toHaveValue('ft');

    await page.getByRole('link', { name: 'Zurück zu den Einstellungen' }).click();
    await page.getByRole('link', { name: 'Zurück zur Flugansicht' }).click();
    await expect(page).toHaveURL('/');
  });
});

test('shows a menu with dedicated settings routes and top back links', async ({ page }) => {
  await page.goto('/settings?testMode=1');

  await expect(page.getByRole('link', { name: 'Language English' })).toBeVisible();
  await expect(
    page.getByRole('link', { name: /^About [A-Z][a-z]{2} \d{1,2}, \d{4}$/ }),
  ).toBeVisible();
  await expect(
    page.getByRole('link', { name: 'Back to flight view' }).locator('.i-mdi-arrow-left'),
  ).toBeVisible();

  let routes = [
    ['Language', '/settings/language'],
    ['Units', '/settings/units'],
    ['Airspace', '/settings/airspace'],
    ['External devices', '/settings/devices'],
    ['About', '/settings/about'],
  ] as const;

  for (let [name, route] of routes) {
    await expect(page.getByRole('link', { name })).toHaveAttribute('href', route);
  }

  for (let [name, route] of routes) {
    await page.getByRole('link', { name }).click();
    await expect(page).toHaveURL(route);
    await expect(page.getByRole('main').locator(':scope > a').first()).toHaveAttribute(
      'href',
      '/settings',
    );
    await page.getByRole('link', { name: 'Back to settings' }).click();
  }
});

test('shows source and build information on the About page', async ({ page }) => {
  await page.goto('/settings/about?testMode=1');

  await expect(page.getByRole('heading', { name: 'About' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'GitHub repository' })).toHaveAttribute(
    'href',
    'https://github.com/Turbo87/updraft',
  );
  await expect(
    page.getByRole('link', { name: EXPECTED_BUILD_COMMIT_SHA.slice(0, 7) }),
  ).toHaveAttribute(
    'href',
    `https://github.com/Turbo87/updraft/commit/${EXPECTED_BUILD_COMMIT_SHA}`,
  );
  let buildTime = page.locator('time');
  await expect(buildTime).toBeVisible();
  expect(Date.parse((await buildTime.getAttribute('datetime')) ?? '')).not.toBeNaN();
  await expect(page.getByRole('heading', { name: 'Data credits' })).not.toBeVisible();
});

test('shows a snapshot of the current map source credits', async ({ page }) => {
  await page.goto('/settings?testMode=1');
  await page.waitForFunction(() =>
    (window as TestWindow).__updraftApp?.mapState.map?.isStyleLoaded(),
  );
  await page.evaluate(() => {
    let map = (window as TestWindow).__updraftApp?.mapState.map;
    if (!map) throw new Error('Map is not available');

    map.addSource('about-page-test', {
      type: 'geojson',
      data: { type: 'FeatureCollection', features: [] },
      attribution: 'Map data <a href="https://example.com/data">Example Data</a>',
    });
  });

  await page.getByRole('link', { name: 'About' }).click();

  await expect(page.getByRole('heading', { name: 'Data credits' })).toBeVisible();
  await expect(page.getByText('Map data')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Example Data' })).toHaveAttribute(
    'href',
    'https://example.com/data',
  );
});

test.describe('with a supported German browser language', () => {
  test.use({ locale: 'de-DE' });

  test('uses German while the backend locale is unset', async ({ page }) => {
    await page.goto('/settings/language?testMode=1');

    await expect(page.getByRole('heading', { name: 'Sprache' })).toBeVisible();
    await expect(page.getByRole('radio', { name: 'Deutsch' })).toBeChecked();

    await page.getByRole('link', { name: 'Zurück zu den Einstellungen' }).click();
    await page.getByRole('link', { name: 'Lufträume' }).click();
    await expect(page.getByText('Keine Luftraumdatei ausgewählt.')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Importieren' })).toBeEnabled();

    await page.getByRole('link', { name: 'Zurück zu den Einstellungen' }).click();
    await page.getByRole('link', { name: 'Einheiten' }).click();
    await expect(page.getByRole('combobox', { name: 'Distanz', exact: true })).toHaveValue('km');
    await expect(page.getByRole('combobox', { name: 'Steigen', exact: true })).toHaveValue('m/s');
  });
});

test('propagates airspace status and invokes import through the fake client', async ({ page }) => {
  await page.goto('/settings/airspace?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await page.evaluate(() => {
    let testWindow = window as TestWindow;
    let client = testWindow.__updraftFake;
    if (!client) throw new Error('the fake client should be available');
    client.importAirspace = async () => {
      testWindow.__airspaceImportCalls = (testWindow.__airspaceImportCalls ?? 0) + 1;
      return { type: 'cancelled' };
    };
  });

  await expect(page.getByText('No airspace file selected.')).toBeVisible();
  await page.getByRole('button', { name: 'Import' }).click();
  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).__airspaceImportCalls))
    .toBe(1);

  await page.evaluate(() => {
    (window as TestWindow).__updraftFake?.emit({
      topic: 'airspace',
      value: {
        type: 'active',
        sourceName: 'rheinland.txt',
        airspaceCount: 42,
        generation: 1,
      },
    });
  });

  await expect(page.getByText('rheinland.txt')).toBeVisible();
  await expect(page.getByText('42 airspaces')).toBeVisible();
});
