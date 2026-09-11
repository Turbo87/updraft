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

it('shows available FlarmNet fields and follows database replacements', async () => {
  let traffic = new TrafficStore();
  let target = {
    id: 'flarm:ABC123',
    position: { latitudeDegrees: 50.82, longitudeDegrees: 6.24 },
    altitudeMslMeters: 1180,
    trafficType: 'glider' as const,
    trackDegrees: 241,
    alarmLevel: 'none' as const,
    stale: false,
  };
  let record = {
    flarmId: 'ABC123',
    callSign: 'EL',
    registration: 'D-TEST',
    planeType: 'AS 33',
    pilotName: 'Example Pilot',
    airfield: 'Example Airfield',
    frequency: '123.450',
  };
  traffic.apply({
    topic: 'traffic',
    value: { type: 'snapshot', value: [{ ...target, flarmnet: record }] },
  });
  await render(TrafficDetails, {
    backLabel: 'Back',
    id: target.id,
    locale: 'en',
    onBack: () => {},
    instruments: new InstrumentsStore(),
    traffic,
    units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
  });
  let region = page.getByRole('region', { name: 'FlarmNet' });
  await expect.element(region).toBeVisible();
  let rows = [...region.element().querySelectorAll('dl > div')].map((row) => [
    row.querySelector('dt')!.textContent,
    row.querySelector('dd')!.textContent,
  ]);
  expect(Object.fromEntries(rows)).toMatchInlineSnapshot(`
    {
      "Aircraft model": "AS 33",
      "Airfield": "Example Airfield",
      "Callsign": "EL",
      "FLARM ID": "ABC123",
      "Frequency": "123.450",
      "Pilot": "Example Pilot",
      "Registration": "D-TEST",
    }
  `);
  traffic.apply({
    topic: 'traffic',
    value: {
      type: 'snapshot',
      value: [
        {
          ...target,
          flarmnet: { ...record, callSign: '', pilotName: '', airfield: '', frequency: '' },
        },
      ],
    },
  });
  await expect.element(region.getByText('Pilot', { exact: true })).not.toBeInTheDocument();
  await expect.element(region.getByText('Callsign', { exact: true })).not.toBeInTheDocument();
  await expect.element(region.getByText('Airfield', { exact: true })).not.toBeInTheDocument();
  await expect.element(region.getByText('Frequency', { exact: true })).not.toBeInTheDocument();
  await expect.element(region.getByText('D-TEST', { exact: true })).toBeVisible();
  traffic.apply({ topic: 'traffic', value: { type: 'snapshot', value: [target] } });
  await expect.element(region).not.toBeInTheDocument();
});

it('shows all signed climb estimates and updates their units and availability', async () => {
  let traffic = new TrafficStore();
  let target = {
    id: 'flarm:ABC123',
    position: { latitudeDegrees: 50.82, longitudeDegrees: 6.24 },
    altitudeMslMeters: 1180,
    trafficType: 'glider' as const,
    trackDegrees: 241,
    alarmLevel: 'none' as const,
    stale: false,
    climb: { average20s: 2, average30s: 0, normalizedEma: -1, smoothed20s: 1 },
  };
  traffic.apply({ topic: 'traffic', value: { type: 'snapshot', value: [target] } });
  let screen = await render(TrafficDetails, {
    backLabel: 'Back',
    id: target.id,
    locale: 'en',
    onBack: () => {},
    instruments: new InstrumentsStore(),
    traffic,
    units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
  });
  function values() {
    return [...document.querySelectorAll('.climb')].map((row) => row.textContent?.trim());
  }
  await expect.poll(values).toEqual(['+2.0 m/s', '0.0 m/s', '-1.0 m/s', '+1.0 m/s']);
  await screen.rerender({
    units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'ft/min' },
  });
  await expect.poll(values).toEqual(['+394 ft/min', '0 ft/min', '-197 ft/min', '+197 ft/min']);
  await screen.rerender({
    units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'kt' },
  });
  traffic.apply({
    topic: 'traffic',
    value: {
      type: 'delta',
      value: {
        upserts: [
          {
            ...target,
            climb: { average20s: -2, average30s: 0.001, normalizedEma: 3, smoothed20s: -1 },
          },
        ],
        removed: [],
      },
    },
  });
  await expect.poll(values).toEqual(['-3.9 kt', '0.0 kt', '+5.8 kt', '-1.9 kt']);

  traffic.apply({
    topic: 'traffic',
    value: { type: 'delta', value: { upserts: [{ ...target, stale: true }], removed: [] } },
  });
  await expect.poll(() => document.querySelectorAll('.climb.stale').length).toBe(4);
  traffic.apply({
    topic: 'traffic',
    value: { type: 'delta', value: { upserts: [], removed: [target.id] } },
  });
  await expect.poll(values).toEqual(['+3.9 kt', '0.0 kt', '-1.9 kt', '+1.9 kt']);
  await expect.poll(() => document.querySelectorAll('.climb.stale').length).toBe(4);

  traffic.apply({
    topic: 'traffic',
    value: { type: 'delta', value: { upserts: [{ ...target, climb: undefined }], removed: [] } },
  });
  await expect.poll(values).toEqual(['—', '—', '—', '—']);
});
