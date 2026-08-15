import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import SettingsIndexScreen from './SettingsIndexScreen.svelte';

describe('SettingsIndexScreen.svelte', () => {
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
      .element(page.getByRole('link', { name: 'Airspace' }))
      .toHaveAttribute('href', '/settings/airspace');
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
