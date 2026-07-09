import { test, expect, type Page } from '@playwright/test';

// A coordinate whose deterministic mock query returns a non-empty feature list.
const COORD = '@52.80000,6.10000';

const openDialog = (page: Page) => page.locator('dialog[open]');
const dialogTitle = (page: Page) => page.locator('dialog .title');
const backButton = (page: Page) => page.getByRole('button', { name: 'Back' });
const closeButton = (page: Page) => page.getByRole('button', { name: 'Close' });

/** Open Settings from the map gear and drill Settings → Map → Orientation (depth 3). */
async function drillToOrientation(page: Page) {
  await page.goto('/');
  await page.waitForSelector('.maplibregl-canvas');
  await page.locator('a.control').click();
  await page.waitForURL('**/settings');
  await page.locator('.panel').getByRole('link', { name: 'Map', exact: true }).click();
  await page.waitForURL('**/settings/map');
  await page.locator('.panel').getByRole('link', { name: 'Orientation' }).click();
  await page.waitForURL('**/settings/map/orientation');
}

test.describe('cold deep-links must not leave re-openable entries behind', () => {
  test('shared list link → drill → close → Back does not re-open', async ({ page }) => {
    await page.goto(`/whats-here/${COORD}`);
    await expect(dialogTitle(page)).toHaveText("What's here?");

    await page.locator('.panel li a').first().click();
    await page.waitForURL(/\/whats-here\/[^/]+\/[^/]+$/);
    await expect(openDialog(page)).toBeVisible();

    await closeButton(page).click();
    await page.waitForURL((u) => u.pathname === '/');
    await expect(openDialog(page)).toHaveCount(0);

    await page.goBack();
    await expect(openDialog(page)).toHaveCount(0); // the fix: Back does not resurrect the dialog
  });

  test('shared feature link → Back → close → Back does not re-open', async ({
    page,
    browser,
    baseURL,
  }) => {
    // Discover a valid feature URL in a throwaway context, so this test's own
    // history stays a clean single cold entry (no extra list entry beneath).
    let scout = await browser.newContext({ baseURL });
    let scoutPage = await scout.newPage();
    await scoutPage.goto(`/whats-here/${COORD}`);
    let featureHref = await scoutPage.locator('.panel li a').first().getAttribute('href');
    await scout.close();
    expect(featureHref).toBeTruthy();

    await page.goto(featureHref!); // cold-load the feature directly
    await expect(page.locator('dialog dl dt').first()).toBeVisible();

    await backButton(page).click();
    await page.waitForURL((u) => u.pathname === `/whats-here/${COORD}`);
    await expect(openDialog(page)).toBeVisible();

    await closeButton(page).click();
    await page.waitForURL((u) => u.pathname === '/');
    await page.goBack();
    await expect(openDialog(page)).toHaveCount(0);
  });

  test('deep settings leaf link → step up twice → close reaches the map', async ({ page }) => {
    await page.goto('/settings/map/orientation');
    await expect(dialogTitle(page)).toHaveText('Orientation');

    await backButton(page).click();
    await page.waitForURL('**/settings/map');
    await expect(dialogTitle(page)).toHaveText('Map');

    await backButton(page).click();
    await page.waitForURL((u) => u.pathname === '/settings');
    await expect(dialogTitle(page)).toHaveText('Settings');

    await closeButton(page).click();
    await page.waitForURL((u) => u.pathname === '/');
    await expect(openDialog(page)).toHaveCount(0);
  });
});

test.describe('warm (in-app) navigation', () => {
  test('hardware Back steps up one level at a time', async ({ page }) => {
    await drillToOrientation(page);
    await expect(dialogTitle(page)).toHaveText('Orientation');

    await page.goBack();
    await page.waitForURL('**/settings/map');
    await expect(dialogTitle(page)).toHaveText('Map');
    await expect(openDialog(page)).toBeVisible();

    await page.goBack();
    await page.waitForURL((u) => u.pathname === '/settings');
    await expect(dialogTitle(page)).toHaveText('Settings');

    await page.goBack();
    await page.waitForURL((u) => u.pathname === '/');
    await expect(openDialog(page)).toHaveCount(0);
  });

  test('Close collapses the whole run; Back does not re-open it', async ({ page }) => {
    await drillToOrientation(page);
    await closeButton(page).click();
    await page.waitForURL((u) => u.pathname === '/');
    await expect(openDialog(page)).toHaveCount(0);

    await page.goBack();
    await expect(openDialog(page)).toHaveCount(0);
  });

  test('the header Back button steps up exactly one level', async ({ page }) => {
    await drillToOrientation(page);
    await backButton(page).click();
    await page.waitForURL('**/settings/map');
    await expect(dialogTitle(page)).toHaveText('Map');
    await expect(openDialog(page)).toBeVisible();
  });
});

test.describe('map integration', () => {
  test('a map tap encodes the real coordinate in the URL', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.maplibregl-canvas');
    await page.mouse.click(450, 300);
    await page.waitForURL(/\/whats-here\/@[-\d.]+,[-\d.]+$/);
    await expect(dialogTitle(page)).toHaveText("What's here?");
  });
});

test.describe('error boundary', () => {
  test('an unknown feature link renders a named error dialog with a working Close', async ({
    page,
  }) => {
    await page.goto(`/whats-here/${COORD}/does-not-exist`);
    await expect(openDialog(page)).toBeVisible();
    await expect(dialogTitle(page)).not.toHaveText(''); // named, not an empty accessible name
    await expect(page.locator('dialog .panel')).toContainText('404');

    await closeButton(page).click();
    await page.waitForURL((u) => u.pathname === '/');
    await expect(openDialog(page)).toHaveCount(0);
  });
});

test.describe('i18n', () => {
  test('dialog titles and screen content are translated via Paraglide', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.maplibregl-canvas');
    await page.getByRole('button', { name: 'DE' }).click(); // switch to German on the map
    await page.locator('a.control').click();
    await page.waitForURL('**/settings');
    await expect(dialogTitle(page)).toHaveText('Einstellungen');
    // Row labels are translated too, not just the title.
    await expect(page.locator('.panel')).toContainText('Karte');

    await page.locator('.panel').getByRole('link', { name: 'Karte', exact: true }).click();
    await page.waitForURL('**/settings/map');
    await expect(dialogTitle(page)).toHaveText('Karte');
    // The Close control's accessible name is translated.
    await expect(page.getByRole('button', { name: 'Schließen' })).toBeVisible();
  });
});
