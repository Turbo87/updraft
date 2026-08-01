import { expect, test } from '@playwright/test';

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
    await expect(page.getByRole('combobox', { name: 'Distanz', exact: true })).toHaveValue('km');
    await expect(page.getByRole('combobox', { name: 'Steigen', exact: true })).toHaveValue('m/s');
  });
});
