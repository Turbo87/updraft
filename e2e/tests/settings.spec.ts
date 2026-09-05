import type { AppContext } from '$lib/app-context';

import { execFileSync } from 'node:child_process';

import { expect, test } from '@playwright/test';

const EXPECTED_BUILD_COMMIT_SHA = execFileSync('git', ['rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();

type TestWindow = Window & {
  __airspaceImportCalls?: number;
  __quitCalls?: number;
  __updraftApp?: AppContext;
  __updraftFake?: {
    emit: (topic: unknown) => void;
    importAirspace: () => Promise<{ type: 'cancelled' }>;
    quit: () => Promise<void>;
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
    let altitude = page.getByRole('group', { name: 'Höhe', exact: true });
    await expect(altitude.getByRole('radio', { name: 'm', exact: true })).toBeChecked();
    await altitude.getByText('ft', { exact: true }).click();
    await expect(altitude.getByRole('radio', { name: 'ft', exact: true })).toBeChecked();

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
    let back = page.getByRole('link', { name: 'Back to settings' });
    await expect(back).toHaveAttribute('href', '/settings');
    await back.click();
  }
});

test('uses the screen scaffold for language settings', async ({ page }) => {
  await page.goto('/settings/language?testMode=1');

  let back = page.getByRole('link', { name: 'Back to settings' });

  await expect(back).toHaveAttribute('href', '/settings');
  await expect(back.locator('.i-mdi-arrow-left')).toBeVisible();
  await expect(page.getByRole('main')).not.toContainText('Back to settings');
});

test('uses the screen scaffold for unit settings', async ({ page }) => {
  await page.goto('/settings/units?testMode=1');

  let back = page.getByRole('link', { name: 'Back to settings' });

  await expect(back).toHaveAttribute('href', '/settings');
  await expect(back.locator('.i-mdi-arrow-left')).toBeVisible();
  await expect(page.getByRole('main')).not.toContainText('Back to settings');
});

test('uses the screen scaffold for airspace settings', async ({ page }) => {
  await page.goto('/settings/airspace?testMode=1');

  let back = page.getByRole('link', { name: 'Back to settings' });

  await expect(back).toHaveAttribute('href', '/settings');
  await expect(back.locator('.i-mdi-arrow-left')).toBeVisible();
  await expect(page.getByRole('main')).not.toContainText('Back to settings');
});

test('uses the screen scaffold when an external device is not found', async ({ page }) => {
  await page.goto('/settings/devices/999?testMode=1');

  let back = page.getByRole('link', { name: 'Back to external devices' });

  await expect(page.getByRole('heading', { name: 'External devices' })).toBeVisible();
  await expect(page.getByText('External device not found')).toBeVisible();
  await expect(back).toHaveAttribute('href', '/settings/devices');
  await expect(page.getByRole('main')).not.toContainText('Back to external devices');
});

test('confirms before quitting through the client from the settings menu', async ({ page }) => {
  await page.goto('/settings?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await page.evaluate(() => {
    let testWindow = window as TestWindow;
    let client = testWindow.__updraftFake;
    if (!client) throw new Error('the fake client should be available');
    client.quit = async () => {
      testWindow.__quitCalls = (testWindow.__quitCalls ?? 0) + 1;
    };
  });

  await expect(page.getByText('Stops background navigation and closes the app.')).toBeVisible();
  await page.getByRole('button', { name: 'Quit Updraft' }).click();

  let dialog = page.getByRole('alertdialog', { name: 'Quit Updraft?' });
  await expect(dialog).toBeVisible();

  await dialog.getByRole('button', { name: 'Cancel' }).click();
  await expect(dialog).not.toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as TestWindow).__quitCalls ?? 0)).toBe(0);

  await page.getByRole('button', { name: 'Quit Updraft' }).click();
  await dialog.getByRole('button', { name: 'Quit Updraft' }).click();
  await expect.poll(() => page.evaluate(() => (window as TestWindow).__quitCalls)).toBe(1);
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
  await expect(page.getByRole('heading', { name: 'Licences' })).toBeVisible();
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
    await expect(
      page.getByRole('group', { name: 'Distanz', exact: true }).getByRole('radio', { name: 'km' }),
    ).toBeChecked();
    await expect(
      page.getByRole('group', { name: 'Steigen', exact: true }).getByRole('radio', { name: 'm/s' }),
    ).toBeChecked();
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

  await expect(page.getByRole('heading', { name: 'Current source' })).toBeVisible();
  await expect(page.getByText('rheinland.txt')).toBeVisible();
  await expect(page.getByText('42', { exact: true })).toBeVisible();
});

test('selects a glide polar and keeps it when revisiting settings', async ({ page }) => {
  await page.goto('/settings?testMode=1');
  await page.getByRole('link', { name: 'Glide', exact: true }).click();
  let polar = page.getByRole('combobox', { name: 'Polar', exact: true });
  await expect(polar).toHaveValue('LS 8');
  await polar.selectOption('LS 8-18');
  await expect(polar).toHaveValue('LS 8-18');
  await page.getByRole('link', { name: 'Back to settings' }).click();
  await page.getByRole('link', { name: 'Glide', exact: true }).click();
  await expect(polar).toHaveValue('LS 8-18');
});
