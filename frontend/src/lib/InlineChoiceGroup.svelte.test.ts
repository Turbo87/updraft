import { describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../app.css';

import InlineChoiceGroup from './InlineChoiceGroup.svelte';

const altitudeOptions = [
  { value: 'm', label: 'm' },
  { value: 'ft', label: 'ft' },
] as const;

describe('InlineChoiceGroup.svelte', () => {
  it('indicates selection without changing the option dimensions or text weight', async () => {
    render(InlineChoiceGroup, {
      name: 'altitude',
      legend: 'Altitude',
      options: altitudeOptions,
      value: 'm',
      onChange: () => {},
    });

    let metres = page.getByText('m', { exact: true });
    let feet = page.getByText('ft', { exact: true });
    let metresLabel = metres.element().closest('label')!;
    let feetLabel = feet.element().closest('label')!;

    expect(getComputedStyle(metresLabel).width).toBe(getComputedStyle(feetLabel).width);
    expect(getComputedStyle(metresLabel).fontWeight).toBe(getComputedStyle(feetLabel).fontWeight);
    expect(getComputedStyle(metresLabel).borderColor).not.toBe(
      getComputedStyle(feetLabel).borderColor,
    );
    expect(metresLabel.querySelector('[class*="i-mdi-check"]')).toBeNull();
  });

  it('reports the selected value through the full label target', async () => {
    let onChange = vi.fn();
    render(InlineChoiceGroup, {
      name: 'altitude',
      legend: 'Altitude',
      options: altitudeOptions,
      value: 'm',
      onChange,
    });

    await page.getByText('ft', { exact: true }).click();

    expect(onChange).toHaveBeenCalledExactlyOnceWith('ft');
  });
});
