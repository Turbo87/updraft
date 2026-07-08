import { page } from 'vitest/browser';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { localStorageKey } from '$lib/paraglide/runtime.js';
import LocaleSwitcher from './LocaleSwitcher.svelte';

describe('LocaleSwitcher.svelte', () => {
  beforeEach(() => {
    localStorage.setItem(localStorageKey, 'en');
  });

  afterEach(() => {
    localStorage.removeItem(localStorageKey);
  });

  it('renders a button per locale, with the active one disabled', async () => {
    render(LocaleSwitcher);

    await expect.element(page.getByRole('button', { name: 'EN' })).toBeDisabled();
    await expect.element(page.getByRole('button', { name: 'DE' })).toBeEnabled();
  });

  it('switches the translated strings in place without a page reload', async () => {
    render(LocaleSwitcher);

    await expect.element(page.getByText('Language')).toBeInTheDocument();

    let nav = document.querySelector('nav')!;
    nav.dataset.reloadCanary = 'alive';

    await page.getByRole('button', { name: 'DE' }).click();

    // Strings re-render reactively and the active button flips...
    await expect.element(page.getByText('Sprache')).toBeInTheDocument();
    await expect.element(page.getByRole('button', { name: 'DE' })).toBeDisabled();
    await expect.element(page.getByRole('button', { name: 'EN' })).toBeEnabled();

    // ...on the original, un-reloaded document: same node, marker intact.
    let nav2 = document.querySelector('nav')!;
    expect(nav2).toBe(nav);
    expect(nav2.dataset.reloadCanary).toBe('alive');
  });
});
