import type { Instruments } from '$lib/protocol/generated/Instruments';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import { EMPTY_DERIVED_INSTRUMENTS, EMPTY_INSTRUMENTS } from '$lib/stores/instruments.svelte';
import MapDebugOverlay from './MapDebugOverlay.svelte';

const emptyInstruments: Instruments = {
  gps: null,
  pressureAltitude: null,
  trueAirspeed: null,
  derived: null,
};

const metricUnits: UnitSettings = {
  altitude: 'm',
  distance: 'km',
  speed: 'km/h',
  verticalSpeed: 'm/s',
};

function text(element: Element): string {
  return element.textContent.replace(/\s+/g, ' ').trim();
}

function readValues(container: HTMLElement): { label: string; value: string }[] {
  let labels = Array.from(container.querySelectorAll('dt'));
  let values = Array.from(container.querySelectorAll('dd'));
  if (labels.length !== values.length) throw new Error('Debug value labels do not match values');

  return labels.map((label, index) => ({
    label: text(label),
    value: text(values[index]),
  }));
}

describe('MapDebugOverlay.svelte', () => {
  it('is hidden until the D key is pressed, then toggles off again', async () => {
    render(MapDebugOverlay, { map: undefined, instruments: emptyInstruments, units: metricUnits });

    let zoom = page.getByText('Zoom');
    await expect.element(zoom).not.toBeInTheDocument();

    await userEvent.keyboard('d');
    await expect.element(zoom).toBeInTheDocument();

    await userEvent.keyboard('d');
    await expect.element(zoom).not.toBeInTheDocument();
  });

  it('offers a tile-boundaries checkbox once visible', async () => {
    render(MapDebugOverlay, { map: undefined, instruments: emptyInstruments, units: metricUnits });

    await userEvent.keyboard('d');

    let checkbox = page.getByRole('checkbox', { name: 'Tile boundaries' });
    await expect.element(checkbox).not.toBeChecked();
    await checkbox.click();
    await expect.element(checkbox).toBeChecked();
  });

  it('offers an independent hit-area checkbox once visible', async () => {
    render(MapDebugOverlay, { map: undefined, instruments: emptyInstruments, units: metricUnits });

    await userEvent.keyboard('d');

    let hitAreas = page.getByRole('checkbox', { name: 'Traffic and waypoint hit areas' });
    let tileBoundaries = page.getByRole('checkbox', { name: 'Tile boundaries' });
    await expect.element(hitAreas).not.toBeChecked();
    await expect.element(tileBoundaries).not.toBeChecked();

    await hitAreas.click();

    await expect.element(hitAreas).toBeChecked();
    await expect.element(tileBoundaries).not.toBeChecked();
  });

  it('keeps missing values and map coordinates unchanged', async () => {
    let view = await render(MapDebugOverlay, {
      map: undefined,
      instruments: emptyInstruments,
      units: metricUnits,
    });

    await userEvent.keyboard('d');

    expect(readValues(view.container)).toMatchInlineSnapshot(`
      [
        {
          "label": "Zoom",
          "value": "0.00",
        },
        {
          "label": "Center",
          "value": "0.00000, 0.00000",
        },
        {
          "label": "Position",
          "value": "–",
        },
        {
          "label": "GPS fix time",
          "value": "–",
        },
        {
          "label": "MSL altitude",
          "value": "–",
        },
        {
          "label": "Ground speed",
          "value": "–",
        },
        {
          "label": "True airspeed",
          "value": "–",
        },
        {
          "label": "Pressure altitude",
          "value": "–",
        },
        {
          "label": "Raw vertical speed",
          "value": "–",
        },
        {
          "label": "Vertical speed",
          "value": "–",
        },
        {
          "label": "Vario",
          "value": "–",
        },
        {
          "label": "Netto",
          "value": "–",
        },
        {
          "label": "Air speed",
          "value": "–",
        },
        {
          "label": "Heading",
          "value": "–",
        },
        {
          "label": "Bank angle",
          "value": "–",
        },
        {
          "label": "Wind",
          "value": "–",
        },
        {
          "label": "Derived altitude",
          "value": "–",
        },
      ]
    `);
  });

  it('shows the current flight values once visible', async () => {
    let instruments: Instruments = {
      gps: {
        position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
        altitudeMeters: 190,
        groundSpeedMetersPerSecond: 30,
        trackDegrees: 45,
        fixTime: { type: 'utcInstant', unixMilliseconds: 1_767_268_800_000 },
        stale: false,
      },
      pressureAltitude: { meters: 1_000, stale: false },
      trueAirspeed: { metersPerSecond: 50, stale: false },
      derived: null,
    };
    let view = await render(MapDebugOverlay, {
      map: undefined,
      instruments,
      units: metricUnits,
    });

    await userEvent.keyboard('d');

    expect(readValues(view.container)).toMatchInlineSnapshot(`
      [
        {
          "label": "Zoom",
          "value": "0.00",
        },
        {
          "label": "Center",
          "value": "0.00000, 0.00000",
        },
        {
          "label": "Position",
          "value": "50.82300, 6.18600",
        },
        {
          "label": "GPS fix time",
          "value": "2026-01-01 12:00:00.000 UTC",
        },
        {
          "label": "MSL altitude",
          "value": "190 m",
        },
        {
          "label": "Ground speed",
          "value": "108.0 km/h",
        },
        {
          "label": "True airspeed",
          "value": "180.0 km/h",
        },
        {
          "label": "Pressure altitude",
          "value": "1000 m",
        },
        {
          "label": "Raw vertical speed",
          "value": "–",
        },
        {
          "label": "Vertical speed",
          "value": "–",
        },
        {
          "label": "Vario",
          "value": "–",
        },
        {
          "label": "Netto",
          "value": "–",
        },
        {
          "label": "Air speed",
          "value": "–",
        },
        {
          "label": "Heading",
          "value": "–",
        },
        {
          "label": "Bank angle",
          "value": "–",
        },
        {
          "label": "Wind",
          "value": "–",
        },
        {
          "label": "Derived altitude",
          "value": "–",
        },
      ]
    `);
    expect(view.container.querySelectorAll('dd.stale')).toHaveLength(0);
  });

  it('shows every estimate the core derived', async () => {
    let instruments: Instruments = {
      ...EMPTY_INSTRUMENTS,
      derived: {
        ...EMPTY_DERIVED_INSTRUMENTS,
        rawVerticalSpeed: { metersPerSecond: 1.7, stale: false },
        verticalSpeed: { metersPerSecond: 1.6, stale: false },
        vario: { metersPerSecond: 1.8, stale: false },
        netto: { metersPerSecond: 2.4, stale: true },
        bank: { angleDegrees: -38, stale: false },
        wind: {
          directionDegrees: 240,
          speedMetersPerSecond: 5.2,
          stale: false,
        },
        heading: { degrees: 265.4, stale: false },
      },
    };
    render(MapDebugOverlay, { map: undefined, instruments, units: metricUnits });

    await userEvent.keyboard('d');

    await expect.element(page.getByText('1.80 m/s', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('1.70 m/s', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('1.60 m/s', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('265°', { exact: true })).toBeInTheDocument();
    let staleNetto = page.getByText('2.40 m/s', { exact: true });
    await expect.element(staleNetto).toHaveClass('stale');
    await expect.element(page.getByText('-38°', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('240° / 18.7 km/h', { exact: true })).toBeInTheDocument();
  });

  it('wraps a rounded wind direction at north', async () => {
    let instruments: Instruments = {
      ...EMPTY_INSTRUMENTS,
      derived: {
        ...EMPTY_DERIVED_INSTRUMENTS,
        wind: {
          directionDegrees: 359.6,
          speedMetersPerSecond: 5.2,
          stale: false,
        },
      },
    };
    render(MapDebugOverlay, { map: undefined, instruments, units: metricUnits });

    await userEvent.keyboard('d');

    await expect.element(page.getByText('0° / 18.7 km/h', { exact: true })).toBeInTheDocument();
  });

  it('wraps a rounded heading at north', async () => {
    let instruments: Instruments = {
      ...EMPTY_INSTRUMENTS,
      derived: {
        ...EMPTY_DERIVED_INSTRUMENTS,
        heading: { degrees: 359.6, stale: false },
      },
    };
    render(MapDebugOverlay, { map: undefined, instruments, units: metricUnits });

    await userEvent.keyboard('d');

    await expect.element(page.getByText('0°', { exact: true })).toBeInTheDocument();
  });

  it('uses the selected vertical-speed unit and stale styling', async () => {
    let instruments: Instruments = {
      ...EMPTY_INSTRUMENTS,
      derived: {
        ...EMPTY_DERIVED_INSTRUMENTS,
        rawVerticalSpeed: { metersPerSecond: 1, stale: true },
        verticalSpeed: { metersPerSecond: 2, stale: false },
      },
    };
    let units: UnitSettings = { ...metricUnits, verticalSpeed: 'ft/min' };
    let view = await render(MapDebugOverlay, { map: undefined, instruments, units });

    await userEvent.keyboard('d');

    await expect.element(page.getByText('196.85 ft/min', { exact: true })).toBeInTheDocument();
    expect(Array.from(view.container.querySelectorAll('dd.stale'), text)).toEqual([
      '196.85 ft/min',
    ]);

    instruments = {
      ...EMPTY_INSTRUMENTS,
      derived: {
        ...EMPTY_DERIVED_INSTRUMENTS,
        rawVerticalSpeed: { metersPerSecond: 1, stale: false },
        verticalSpeed: { metersPerSecond: 2, stale: true },
        vario: { metersPerSecond: 3, stale: true },
      },
    };
    await view.rerender({ map: undefined, instruments, units });

    await expect.element(page.getByText('393.70 ft/min', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('590.55 ft/min', { exact: true })).toBeInTheDocument();
    expect(Array.from(view.container.querySelectorAll('dd.stale'), text)).toEqual([
      '393.70 ft/min',
      '590.55 ft/min',
    ]);
  });

  it('uses the selected altitude and speed units', async () => {
    let instruments: Instruments = {
      gps: {
        position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
        altitudeMeters: 190,
        groundSpeedMetersPerSecond: 30,
        trackDegrees: 45,
        fixTime: { type: 'utcTimeOfDay', millisecondsSinceMidnight: 43_201_250 },
        stale: true,
      },
      pressureAltitude: { meters: 1_000, stale: true },
      trueAirspeed: { metersPerSecond: 50, stale: true },
      derived: null,
    };
    let units: UnitSettings = {
      altitude: 'ft',
      distance: 'nm',
      speed: 'kt',
      verticalSpeed: 'ft/min',
    };
    let view = await render(MapDebugOverlay, { map: undefined, instruments, units });

    await userEvent.keyboard('d');

    expect(readValues(view.container)).toMatchInlineSnapshot(`
      [
        {
          "label": "Zoom",
          "value": "0.00",
        },
        {
          "label": "Center",
          "value": "0.00000, 0.00000",
        },
        {
          "label": "Position",
          "value": "50.82300, 6.18600",
        },
        {
          "label": "GPS fix time",
          "value": "12:00:01.250 UTC",
        },
        {
          "label": "MSL altitude",
          "value": "623 ft",
        },
        {
          "label": "Ground speed",
          "value": "58.3 kt",
        },
        {
          "label": "True airspeed",
          "value": "97.2 kt",
        },
        {
          "label": "Pressure altitude",
          "value": "3281 ft",
        },
        {
          "label": "Raw vertical speed",
          "value": "–",
        },
        {
          "label": "Vertical speed",
          "value": "–",
        },
        {
          "label": "Vario",
          "value": "–",
        },
        {
          "label": "Netto",
          "value": "–",
        },
        {
          "label": "Air speed",
          "value": "–",
        },
        {
          "label": "Heading",
          "value": "–",
        },
        {
          "label": "Bank angle",
          "value": "–",
        },
        {
          "label": "Wind",
          "value": "–",
        },
        {
          "label": "Derived altitude",
          "value": "–",
        },
      ]
    `);
    expect(Array.from(view.container.querySelectorAll('dd.stale'), text)).toEqual([
      '50.82300, 6.18600',
      '12:00:01.250 UTC',
      '623 ft',
      '58.3 kt',
      '97.2 kt',
      '3281 ft',
    ]);
  });
});
