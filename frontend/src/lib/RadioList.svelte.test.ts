import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import RadioList from './RadioList.svelte';

const distanceOptions = [
  { value: 'km', label: 'Kilometres · km' },
  { value: 'mi', label: 'Miles · mi' },
  { value: 'nm', label: 'Nautical miles · nm' },
] as const;

const languageOptions = [
  { value: 'en', label: 'English', icon: 'i-circle-flags-lang-en' },
  { value: 'de', label: 'Deutsch', icon: 'i-circle-flags-lang-de' },
] as const;

describe('RadioList.svelte', () => {
  it('renders one card of native radio targets without changing selected text weight', async () => {
    render(RadioList, {
      name: 'distance',
      legend: 'Distance',
      options: distanceOptions,
      value: 'km',
      onChange: () => {},
    });

    let kilometres = page.getByRole('radio', { name: 'Kilometres · km' });
    let miles = page.getByRole('radio', { name: 'Miles · mi' });
    await expect.element(kilometres).toBeChecked();
    expect(getComputedStyle(kilometres.element()).width).toBe('24px');
    expect(getComputedStyle(kilometres.element().closest('label')!).minHeight).toBe('48px');
    expect(getComputedStyle(kilometres.element().closest('label')!).fontWeight).toBe(
      getComputedStyle(miles.element().closest('label')!).fontWeight,
    );
  });

  it('reports the selected value through the full label target', async () => {
    let onChange = vi.fn();
    render(RadioList, {
      name: 'distance',
      legend: 'Distance',
      options: distanceOptions,
      value: 'km',
      onChange,
    });

    await page.getByText('Miles · mi', { exact: true }).click();

    expect(onChange).toHaveBeenCalledExactlyOnceWith('mi');
  });

  it('renders decorative icons supplied by the options', () => {
    render(RadioList, {
      name: 'language',
      legend: 'Language',
      hideLegend: true,
      options: languageOptions,
      value: 'en',
      onChange: () => {},
    });

    let icon = document.querySelector('.i-circle-flags-lang-en');
    let legend = page.getByText('Language', { exact: true });

    expect(icon).not.toBeNull();
    expect(icon).toHaveAttribute('aria-hidden', 'true');
    expect(getComputedStyle(legend.element()).position).toBe('absolute');
  });
});
