import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import GlideSettings from './GlideSettings.svelte';

describe('GlideSettings.svelte', () => {
  it('allows retry after the catalog fails to load', async () => {
    let getPolars = vi.fn().mockRejectedValueOnce(new Error('driver stopped'));
    getPolars.mockResolvedValueOnce(['LS 8', 'LS 8-18']);
    render(GlideSettings, { polar: 'LS 8', getPolars, setPolar: vi.fn() });

    await expect.element(page.getByRole('alert')).toHaveTextContent('Could not load polars.');
    await page.getByRole('button', { name: 'Retry' }).click();
    await expect.element(page.getByRole('combobox', { name: 'Polar' })).toHaveValue('LS 8');
    await expect.element(page.getByRole('alert')).not.toBeInTheDocument();
  });

  it('restores the authoritative selection and allows retry after a rejected change', async () => {
    let setPolar = vi.fn().mockRejectedValue(new Error('driver stopped'));
    render(GlideSettings, {
      polar: 'LS 8',
      getPolars: async () => ['LS 8', 'LS 8-18'],
      setPolar,
    });
    let select = page.getByRole('combobox', { name: 'Polar' });
    await select.selectOptions('LS 8-18');

    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Could not change the selected polar.');
    await expect.element(select).toHaveValue('LS 8');
    await expect.element(select).toBeEnabled();
    expect(setPolar).toHaveBeenCalledExactlyOnceWith('LS 8-18');
  });
});
