import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import SettingsIndexScreen from './SettingsIndexScreen.svelte';

describe('SettingsIndexScreen.svelte', () => {
  it.each([320, 413, 915])('keeps separate inset navigation cards at width %s', async (width) => {
    let oldWidth = window.innerWidth;
    let oldHeight = window.innerHeight;
    try {
      await page.viewport(width, 600);
      render(SettingsIndexScreen, { updateCount: 2 });
      let nav = page.getByRole('navigation', { name: 'Settings' }).element();
      let navBounds = nav.getBoundingClientRect();
      let links = [...nav.querySelectorAll('a')];
      expect(links).toHaveLength(8);
      expect(navBounds.left).toBe((width - Math.min(width, 544)) / 2 + 20);
      expect(navBounds.right).toBe(width - navBounds.left);
      for (let [index, link] of links.entries()) {
        let bounds = link.getBoundingClientRect();
        expect([bounds.left, bounds.right, bounds.height]).toEqual([
          navBounds.left,
          navBounds.right,
          56,
        ]);
        expect(getComputedStyle(link).borderRadius).toBe('12px');
        if (index > 0) {
          expect(bounds.top - links[index - 1].getBoundingClientRect().bottom).toBe(8);
        }
      }
    } finally {
      await page.viewport(oldWidth, oldHeight);
    }
  });

  it('links to every settings section and back to the Flight View', async () => {
    render(SettingsIndexScreen, {
      buildDate: '15 Aug 2026',
      language: 'English',
    });

    await expect
      .element(page.getByRole('link', { name: 'Back to flight view' }))
      .toHaveAttribute('href', '/');
    await expect
      .element(page.getByRole('link', { name: 'Language English' }))
      .toHaveAttribute('href', '/settings/language');
    await expect
      .element(page.getByRole('link', { name: 'Units' }))
      .toHaveAttribute('href', '/settings/units');
    await expect
      .element(page.getByRole('link', { name: 'Flight controls' }))
      .toHaveAttribute('href', '/settings/flight-controls');
    await expect
      .element(page.getByRole('link', { name: 'Data', exact: true }))
      .toHaveAttribute('href', '/settings/data');
    await expect
      .element(page.getByRole('link', { name: 'Waypoints', exact: true }))
      .not.toBeInTheDocument();
    await expect
      .element(page.getByRole('link', { name: 'Vario', exact: true }))
      .toHaveAttribute('href', '/settings/vario');
    await expect.element(page.getByRole('spinbutton')).not.toBeInTheDocument();
    await expect.element(page.getByRole('link', { name: 'Airspace' })).not.toBeInTheDocument();
    await expect
      .element(page.getByRole('link', { name: 'External devices' }))
      .toHaveAttribute('href', '/settings/devices');
    await expect
      .element(page.getByRole('link', { name: 'About 15 Aug 2026' }))
      .toHaveAttribute('href', '/settings/about');
  });

  it('keeps unavailable language and build date visible as unknown values', () => {
    render(SettingsIndexScreen, {});

    expect(document.querySelectorAll('.value')).toHaveLength(2);
    expect([...document.querySelectorAll('.value')].map((value) => value.textContent)).toEqual([
      '—',
      '—',
    ]);
  });
});

it('shows the update count and removes it when no updates remain', async () => {
  let screen = await render(SettingsIndexScreen, { updateCount: 2 });
  await expect
    .element(page.getByRole('link', { name: 'Data 2 updates', exact: true }))
    .toHaveAttribute('href', '/settings/data');
  await screen.rerender({ updateCount: 1 });
  await expect
    .element(page.getByRole('link', { name: 'Data 1 update', exact: true }))
    .toBeVisible();
  await screen.rerender({ updateCount: 0 });
  await expect.element(page.getByRole('link', { name: 'Data', exact: true })).toBeVisible();
});
