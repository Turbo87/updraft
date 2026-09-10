import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../../../app.css';

import Infobox from './Infobox.svelte';

describe('Infobox', () => {
  it('ellipsizes long labels and gives the value more vertical space', async () => {
    let label = 'Geschwindigkeit über Grund';
    let view = await render(Infobox, {
      label,
      value: { kind: 'speed', metersPerSecond: 30, unit: 'km/h' },
      stale: false,
    });
    let box = view.container.querySelector<HTMLElement>('.infobox')!;
    box.style.width = '71px';
    box.style.height = '71px';
    await document.fonts.ready;
    let heading = box.querySelector<HTMLElement>('.label')!;
    expect(getComputedStyle(heading).textOverflow).toBe('ellipsis');
    expect(getComputedStyle(heading).whiteSpace).toBe('nowrap');
    expect(heading.scrollWidth).toBeGreaterThan(heading.clientWidth);
    let [labelHeight, valueHeight] = getComputedStyle(box)
      .gridTemplateRows.split(' ')
      .map(parseFloat);
    expect(valueHeight).toBeGreaterThan(labelHeight * 2);
    await expect.element(page.getByRole('group', { name: label })).toBeVisible();
  });

  it('fits long altitude and climb readouts in a phone-width cell', async () => {
    let view = await render(Infobox, {
      label: 'Altitude',
      value: { kind: 'altitude', meters: 3048, unit: 'ft' },
      stale: false,
    });
    let box = view.container.querySelector<HTMLElement>('.infobox')!;
    box.style.width = '71px';
    box.style.height = '64px';
    await document.fonts.ready;
    let readout = box.querySelector<HTMLElement>('.numeric-value')!;
    expect(readout.getBoundingClientRect().right).toBeLessThanOrEqual(
      box.getBoundingClientRect().right - 4,
    );
    await view.rerender({
      value: { kind: 'vertical-speed', metersPerSecond: -5.08, unit: 'ft/min' },
    });
    expect(readout.getBoundingClientRect().right).toBeLessThanOrEqual(
      box.getBoundingClientRect().right - 4,
    );
  });
  it('converts canonical altitude and updates when the unit changes', async () => {
    let view = await render(Infobox, {
      label: 'Altitude',
      value: { kind: 'altitude', meters: 304.8, unit: 'm' },
      stale: false,
    });
    let box = page.getByRole('group', { name: 'Altitude' });
    await expect.element(box).toHaveTextContent('305 m');

    await view.rerender({ value: { kind: 'altitude', meters: 304.8, unit: 'ft' } });
    await expect.element(box).toHaveTextContent('1000 ft');
  });

  it('keeps stale values but removes the unit for missing values', async () => {
    let view = await render(Infobox, {
      label: 'TAS',
      value: { kind: 'speed', metersPerSecond: 30, unit: 'km/h' },
      stale: true,
    });
    let box = page.getByRole('group', { name: 'TAS' });
    await expect.element(box).toHaveTextContent('108');
    expect(view.container.querySelector('.numeric-value')?.classList.contains('stale')).toBe(true);
    expect(view.container.querySelector('.unit-label')?.textContent).toBe('km/h');
    await view.rerender({ value: { kind: 'speed', metersPerSecond: null, unit: 'km/h' } });
    await expect.element(box).toHaveTextContent('–');
    expect(view.container.querySelector('.unit')).toBeNull();
  });

  it.each([
    [-12, true, false],
    [12, false, true],
    [-0.4, true, true],
    [null, false, false],
  ] as const)('shows directional chevrons for %s degrees', async (degrees, left, right) => {
    let view = await render(Infobox, {
      label: 'Bank angle',
      value: { kind: 'relative-angle', degrees },
      stale: false,
    });
    expect(view.container.querySelector('.chevron-left') !== null).toBe(left);
    expect(view.container.querySelector('.chevron-right') !== null).toBe(right);
    await expect
      .element(page.getByRole('group', { name: 'Bank angle' }))
      .toHaveTextContent(degrees === null ? '–' : degrees === -0.4 ? '0' : '12');
  });
});
