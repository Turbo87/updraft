import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import LanguageSetting from './LanguageSetting.svelte';

describe('LanguageSetting.svelte', () => {
  it('shows English and German with the active locale selected', async () => {
    render(LanguageSetting, {
      locale: 'en',
      onLocaleChange: () => {},
    });

    await expect.element(page.getByRole('radio', { name: 'English' })).toBeChecked();
    await expect.element(page.getByRole('radio', { name: 'Deutsch' })).not.toBeChecked();
    expect(document.querySelectorAll('.flag')).toHaveLength(2);
  });

  it('reports and optimistically selects the chosen locale', async () => {
    let onLocaleChange = vi.fn();
    render(LanguageSetting, {
      locale: 'en',
      onLocaleChange,
    });

    await page.getByRole('radio', { name: 'Deutsch' }).click();

    expect(onLocaleChange).toHaveBeenCalledExactlyOnceWith('de');
    await expect.element(page.getByRole('radio', { name: 'Deutsch' })).toBeChecked();
  });
});
