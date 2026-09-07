import type { Map } from 'maplibre-gl';

import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../../../app.css';

import { AIRSPACE_BROWSER_FIXTURE } from '$lib/map/airspace.fixture';
import AirspaceDetails from './AirspaceDetails.svelte';

const map = {
  on() {},
  off() {},
  isSourceLoaded: () => true,
  getSource: () => ({ getData: async () => AIRSPACE_BROWSER_FIXTURE }),
} as unknown as Map;

it.each([413, 544, 915])('lays out airspace detail cards at width %s', async (width) => {
  let oldWidth = window.innerWidth;
  let oldHeight = window.innerHeight;
  let root = document.documentElement;
  let previousStyle = root.getAttribute('style');
  try {
    await page.viewport(width, 600);
    root.style.setProperty('--safe-area-left', '24px');
    root.style.setProperty('--safe-area-right', '12px');
    render(AirspaceDetails, {
      altitudeUnit: 'm',
      backLabel: 'Back',
      id: '1:0:0',
      locale: 'en',
      map,
      onBack: () => {},
    });
    await expect.element(page.getByRole('heading', { name: 'Düsseldorf CTR' })).toBeVisible();
    let lists = [...document.querySelectorAll('dl.detail-card, ul.country-card')];
    expect(lists).toHaveLength(7);
    for (let list of lists) {
      let bounds = list.getBoundingClientRect();
      let sectionBounds = list.closest('section')!.getBoundingClientRect();
      expect([bounds.left, bounds.right]).toEqual(
        width <= 544 ? [0, width] : [sectionBounds.left, sectionBounds.right],
      );
      let content = list.matches('ul') ? list : list.firstElementChild!;
      expect([
        getComputedStyle(content).paddingLeft,
        getComputedStyle(content).paddingRight,
      ]).toEqual(width <= 544 ? ['44px', '32px'] : ['20px', '20px']);
      if (list.matches('dl') && list.children.length > 1) {
        expect(getComputedStyle(list.children[1]).borderTopWidth).toBe('1px');
      }
    }
    let summary = document.querySelector('.summary')!;
    let summaryBounds = summary.getBoundingClientRect();
    let firstSection = lists[0].closest('section')!.getBoundingClientRect();
    expect([summaryBounds.left, summaryBounds.right]).toEqual([
      firstSection.left,
      firstSection.right,
    ]);
    expect(firstSection.top - summaryBounds.bottom).toBe(24);
    let remarks = page.getByText('ACTIVE DURING GLIDER EVENTS').element();
    let remarksBounds = remarks.getBoundingClientRect();
    expect([remarksBounds.left, remarksBounds.right]).toEqual([
      firstSection.left,
      firstSection.right,
    ]);
  } finally {
    if (previousStyle === null) root.removeAttribute('style');
    else root.setAttribute('style', previousStyle);
    await page.viewport(oldWidth, oldHeight);
  }
});
