import { test, expect, type Page } from '@playwright/test';

// End-to-end coverage of the route-driven dialog wiring, driven against the demo
// chain map -> /menu -> /menu/settings -> /menu/settings/language. The reducer's
// pure decisions are unit-tested in src/lib/dialog/navigation.test.ts; these tests
// exercise the history wiring (stamped page.state, history.go, popstate, cold
// deep-links) against a real browser history stack.

const openDialog = (page: Page) => page.locator('dialog[open]');
const dialogTitle = (page: Page) => page.locator('dialog .title');
const backButton = (page: Page) => page.getByRole('button', { name: 'Back' });
const closeButton = (page: Page) => page.getByRole('button', { name: 'Close' });
const panelLink = (page: Page, name: string) =>
  page.locator('dialog .panel').getByRole('link', { name });

async function openMenu(page: Page) {
  await page.goto('/');
  await page.waitForSelector('.maplibregl-canvas');
  await page.getByRole('link', { name: 'Menu' }).click();
  await page.waitForURL('**/menu');
}

/** Drill map -> Menu -> Settings -> Language (depth 3). */
async function drillToLanguage(page: Page) {
  await openMenu(page);
  await panelLink(page, 'Settings').click();
  await page.waitForURL('**/menu/settings');
  await panelLink(page, 'Language').click();
  await page.waitForURL('**/menu/settings/language');
}

test.describe('warm (in-app) navigation', () => {
  test('hardware Back steps up one level at a time', async ({ page }) => {
    await drillToLanguage(page);
    await expect(dialogTitle(page)).toHaveText('Language');

    await page.goBack();
    await page.waitForURL('**/menu/settings');
    await expect(dialogTitle(page)).toHaveText('Settings');
    await expect(openDialog(page)).toBeVisible();

    await page.goBack();
    await page.waitForURL((u) => u.pathname === '/menu');
    await expect(dialogTitle(page)).toHaveText('Menu');

    await page.goBack();
    await page.waitForURL((u) => u.pathname === '/');
    await expect(openDialog(page)).toHaveCount(0);
  });

  test('Close collapses the whole run; Back does not re-open it', async ({ page }) => {
    await drillToLanguage(page);
    await closeButton(page).click();
    await page.waitForURL((u) => u.pathname === '/');
    await expect(openDialog(page)).toHaveCount(0);

    await page.goBack();
    await expect(openDialog(page)).toHaveCount(0);
  });

  test('the header Back button steps up exactly one level', async ({ page }) => {
    await drillToLanguage(page);
    await backButton(page).click();
    await page.waitForURL('**/menu/settings');
    await expect(dialogTitle(page)).toHaveText('Settings');
    await expect(openDialog(page)).toBeVisible();
  });

  test('drilling to a new screen moves focus off the tapped link', async ({ page }) => {
    await openMenu(page);
    await panelLink(page, 'Settings').click();
    await page.waitForURL('**/menu/settings');
    // Focus must not be stranded on the removed link's fallback (document.body):
    // a screen reader has to land on the new screen. The heading receives it.
    await expect(page.locator('dialog .title')).toBeFocused();
  });
});

test.describe('cold deep-links must not leave re-openable entries behind', () => {
  test('cold single entry: Close reaches the map and Back does not re-open', async ({ page }) => {
    await page.goto('/menu/settings/language');
    await expect(dialogTitle(page)).toHaveText('Language');

    await closeButton(page).click();
    await page.waitForURL((u) => u.pathname === '/');
    await expect(openDialog(page)).toHaveCount(0);

    await page.goBack();
    await expect(openDialog(page)).toHaveCount(0);
  });

  test('cold single entry: Back reaches the map (nothing beneath to step up to)', async ({
    page,
  }) => {
    await page.goto('/menu/settings');
    await expect(dialogTitle(page)).toHaveText('Settings');

    await backButton(page).click();
    await page.waitForURL((u) => u.pathname === '/');
    await expect(openDialog(page)).toHaveCount(0);
  });

  test('cold + drilled: Close collapses cold entries; Back does not re-open', async ({ page }) => {
    await page.goto('/menu/settings'); // cold entry, depth 1
    await panelLink(page, 'Language').click();
    await page.waitForURL('**/menu/settings/language'); // depth 2

    await closeButton(page).click();
    await page.waitForURL((u) => u.pathname === '/');
    await expect(openDialog(page)).toHaveCount(0);

    await page.goBack();
    await expect(openDialog(page)).toHaveCount(0);
  });
});

test.describe('map integration', () => {
  test('the menu button opens the menu dialog', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.maplibregl-canvas');
    await page.getByRole('link', { name: 'Menu' }).click();
    await page.waitForURL((u) => u.pathname === '/menu');
    await expect(dialogTitle(page)).toHaveText('Menu');
  });
});

test.describe('i18n', () => {
  test('switching locale in the language screen translates the chrome', async ({ page }) => {
    await drillToLanguage(page);
    await page.getByRole('button', { name: 'DE' }).click();

    await expect(dialogTitle(page)).toHaveText('Sprache');
    // The Close control's accessible name is translated too.
    await expect(page.getByRole('button', { name: 'Schließen' })).toBeVisible();

    await page.getByRole('button', { name: 'Zurück' }).click();
    await page.waitForURL('**/menu/settings');
    await expect(dialogTitle(page)).toHaveText('Einstellungen');
  });
});
