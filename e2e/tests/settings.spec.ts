import { expect, test } from '@playwright/test';

type TestWindow = Window & {
  __airspaceImportCalls?: number;
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

    await expect(page).toHaveURL(/\/settings$/);
    await expect(page.getByRole('radio', { name: 'English' })).toBeChecked();

    let altitude = page.getByRole('combobox', { name: 'Altitude', exact: true });
    await expect(altitude).toHaveValue('m');
    await altitude.selectOption('ft');
    await expect(altitude).toHaveValue('ft');

    await page.getByRole('radio', { name: 'Deutsch' }).click();

    await expect(page.getByRole('heading', { name: 'Einstellungen' })).toBeVisible();
    await expect(page.getByRole('radio', { name: 'Deutsch' })).toBeChecked();
    await expect(page.locator('html')).toHaveAttribute('lang', 'de');

    await page.getByRole('link', { name: 'Zurück zur Flugansicht' }).click();
    await expect(page).toHaveURL('/');
  });
});

test.describe('with a supported German browser language', () => {
  test.use({ locale: 'de-DE' });

  test('uses German while the backend locale is unset', async ({ page }) => {
    await page.goto('/settings?testMode=1');

    await expect(page.getByRole('heading', { name: 'Einstellungen' })).toBeVisible();
    await expect(page.getByRole('radio', { name: 'Deutsch' })).toBeChecked();
    await expect(page.getByText('Keine Luftraumdatei ausgewählt.')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Importieren' })).toBeEnabled();
    await expect(page.getByRole('combobox', { name: 'Distanz', exact: true })).toHaveValue('km');
    await expect(page.getByRole('combobox', { name: 'Steigen', exact: true })).toHaveValue('m/s');
  });
});

test('propagates airspace status and invokes import through the fake client', async ({ page }) => {
  await page.goto('/settings?testMode=1');
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
