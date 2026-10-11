import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import MainMenuScreen from './MainMenuScreen.svelte';

describe('MainMenuScreen.svelte', () => {
  it('links to every menu section and back to the Flight View', async () => {
    render(MainMenuScreen, { buildDate: '15 Aug 2026' });

    await expect
      .element(page.getByRole('heading', { name: 'Main menu', exact: true }))
      .toBeVisible();
    await expect
      .element(page.getByRole('link', { name: 'Back to flight view' }))
      .toHaveAttribute('href', '/');
    await expect
      .element(page.getByRole('link', { name: 'Navigation' }))
      .toHaveAttribute('href', '/navigation');
    await expect
      .element(page.getByRole('link', { name: 'Flight controls' }))
      .toHaveAttribute('href', '/flight-controls');
    await expect
      .element(page.getByRole('link', { name: 'Settings' }))
      .toHaveAttribute('href', '/settings');
    await expect
      .element(page.getByRole('link', { name: 'About 15 Aug 2026' }))
      .toHaveAttribute('href', '/about');
    expect(
      page.getByRole('navigation', { name: 'Main menu' }).element().querySelectorAll('a'),
    ).toHaveLength(4);
  });

  it('keeps an unavailable build date visible as an unknown value', async () => {
    render(MainMenuScreen, {});

    await expect.element(page.getByRole('link', { name: 'About —' })).toBeVisible();
  });

  it('calls the quit handler from the quit action', async () => {
    let onQuit = vi.fn();
    render(MainMenuScreen, { onQuit });

    await page.getByRole('button', { name: 'Quit Updraft' }).click();

    expect(onQuit).toHaveBeenCalledOnce();
  });

  it('calls the new flight handler from the new flight action', async () => {
    let onNewFlight = vi.fn();
    render(MainMenuScreen, { onNewFlight });

    await page.getByRole('button', { name: 'New flight' }).click();

    expect(onNewFlight).toHaveBeenCalledOnce();
  });

  it('hides the new flight action without a handler', async () => {
    render(MainMenuScreen, {});

    await expect.element(page.getByRole('heading', { name: 'Main menu' })).toBeVisible();
    await expect.element(page.getByRole('button', { name: 'New flight' })).not.toBeInTheDocument();
  });
});
