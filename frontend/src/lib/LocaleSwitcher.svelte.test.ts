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
});
