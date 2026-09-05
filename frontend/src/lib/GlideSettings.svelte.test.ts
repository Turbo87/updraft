import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import GlideSettings from './GlideSettings.svelte';

const reserveProps = {
  arrivalReserve: 200,
  altitudeUnit: 'm' as const,
  setArrivalReserve: vi.fn(),
};

describe('GlideSettings.svelte', () => {
  it('allows retry after the catalog fails to load', async () => {
    let getPolars = vi.fn().mockRejectedValueOnce(new Error('driver stopped'));
    getPolars.mockResolvedValueOnce(['LS 8', 'LS 8-18']);
    render(GlideSettings, { ...reserveProps, polar: 'LS 8', getPolars, setPolar: vi.fn() });

    await expect.element(page.getByRole('alert')).toHaveTextContent('Could not load polars.');
    await page.getByRole('button', { name: 'Retry' }).click();
    await expect.element(page.getByRole('combobox', { name: 'Polar' })).toHaveValue('LS 8');
    await expect.element(page.getByRole('alert')).not.toBeInTheDocument();
  });

  it('restores the authoritative selection and allows retry after a rejected change', async () => {
    let setPolar = vi.fn().mockRejectedValue(new Error('driver stopped'));
    render(GlideSettings, {
      ...reserveProps,
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

  it('displays feet and saves fractional meters only after an edit', async () => {
    let setArrivalReserve = vi.fn();
    render(GlideSettings, {
      ...reserveProps,
      altitudeUnit: 'ft',
      setArrivalReserve,
      polar: 'LS 8',
      getPolars: async () => ['LS 8'],
      setPolar: vi.fn(),
    });
    let input = page.getByRole('spinbutton', { name: 'Arrival reserve (ft)' });
    await expect.element(input).toHaveValue(656);
    await input.click();
    await page.getByRole('heading').click();
    expect(setArrivalReserve).not.toHaveBeenCalled();
    await input.fill('1000');
    await page.getByRole('heading').click();
    expect(setArrivalReserve).toHaveBeenCalledExactlyOnceWith(304.8);
  });

  it('rejects negative reserves and restores the saved value after command failure', async () => {
    let setArrivalReserve = vi.fn().mockRejectedValue(new Error('driver stopped'));
    render(GlideSettings, {
      ...reserveProps,
      setArrivalReserve,
      polar: 'LS 8',
      getPolars: async () => ['LS 8'],
      setPolar: vi.fn(),
    });
    let input = page.getByRole('spinbutton', { name: 'Arrival reserve (m)' });
    await input.fill('-1');
    await page.getByRole('heading').click();
    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Enter a nonnegative arrival reserve.');
    expect(setArrivalReserve).not.toHaveBeenCalled();
    await input.fill('300');
    await page.getByRole('heading').click();
    await expect
      .element(page.getByRole('alert'))
      .toHaveTextContent('Could not change the arrival reserve.');
    await expect.element(input).toHaveValue(200);
    await expect.element(input).toBeEnabled();
  });
});
