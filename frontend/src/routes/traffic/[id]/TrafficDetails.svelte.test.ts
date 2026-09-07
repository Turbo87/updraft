import { expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import '../../../app.css';

import { InstrumentsStore } from '$lib/stores/instruments.svelte';
import { TrafficStore } from '$lib/stores/traffic.svelte';
import TrafficDetails from './TrafficDetails.svelte';

it.each([413, 544, 915])('lays out traffic detail cards at width %s', async (width) => {
  let oldWidth = window.innerWidth;
  let oldHeight = window.innerHeight;
  let root = document.documentElement;
  let previousStyle = root.getAttribute('style');
  try {
    await page.viewport(width, 600);
    root.style.setProperty('--safe-area-left', '24px');
    root.style.setProperty('--safe-area-right', '12px');
    let traffic = new TrafficStore();
    traffic.apply({
      topic: 'traffic',
      value: {
        type: 'snapshot',
        value: [
          {
            id: 'flarm:ABC123',
            position: { latitudeDegrees: 50.82, longitudeDegrees: 6.24 },
            altitudeMslMeters: 1180,
            trafficType: 'glider',
            trackDegrees: 241,
            alarmLevel: 'none',
            stale: false,
          },
        ],
      },
    });
    await render(TrafficDetails, {
      backLabel: 'Back',
      id: 'flarm:ABC123',
      locale: 'en',
      onBack: () => {},
      instruments: new InstrumentsStore(),
      traffic,
      units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
    });
    let lists = [...document.querySelectorAll('dl')];
    expect(lists).toHaveLength(2);
    for (let list of lists) {
      let bounds = list.getBoundingClientRect();
      let sectionBounds = list.closest('section')!.getBoundingClientRect();
      expect([bounds.left, bounds.right]).toEqual(
        width <= 544 ? [0, width] : [sectionBounds.left, sectionBounds.right],
      );
      let row = list.children[1];
      expect([getComputedStyle(row).paddingLeft, getComputedStyle(row).paddingRight]).toEqual(
        width <= 544 ? ['44px', '32px'] : ['20px', '20px'],
      );
      expect(getComputedStyle(row).borderTopWidth).toBe('1px');
    }
    let summaryBounds = document.querySelector('.summary')!.getBoundingClientRect();
    let sectionBounds = lists[0].closest('section')!.getBoundingClientRect();
    expect([summaryBounds.left, summaryBounds.right]).toEqual([
      sectionBounds.left,
      sectionBounds.right,
    ]);
    expect(sectionBounds.top - summaryBounds.bottom).toBe(24);
  } finally {
    if (previousStyle === null) root.removeAttribute('style');
    else root.setAttribute('style', previousStyle);
    await page.viewport(oldWidth, oldHeight);
  }
});
