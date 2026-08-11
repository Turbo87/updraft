import type { Instruments } from '$lib/protocol/generated/Instruments';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import { EMPTY_AIR_ESTIMATE, EMPTY_INSTRUMENTS } from '$lib/stores/instruments.svelte';
import MapDebugOverlay from './MapDebugOverlay.svelte';

const emptyInstruments: Instruments = {
  gps: null,
  pressureAltitude: null,
  trueAirspeed: null,
  air: null,
};

const metricUnits: UnitSettings = {
  altitude: 'm',
  distance: 'km',
  speed: 'km/h',
  verticalSpeed: 'm/s',
};

function readValues(container: HTMLElement): { label: string; value: string }[] {
  let labels = Array.from(container.querySelectorAll('dt'));
  let values = Array.from(container.querySelectorAll('dd'));
  if (labels.length !== values.length) throw new Error('Debug value labels do not match values');

  function text(element: Element): string {
    return element.textContent.replace(/\s+/g, ' ').trim();
  }

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

  it('offers an independent traffic-hit-area checkbox once visible', async () => {
    render(MapDebugOverlay, { map: undefined, instruments: emptyInstruments, units: metricUnits });

    await userEvent.keyboard('d');

    let trafficHitAreas = page.getByRole('checkbox', { name: 'Traffic hit areas' });
    let tileBoundaries = page.getByRole('checkbox', { name: 'Tile boundaries' });
    await expect.element(trafficHitAreas).not.toBeChecked();
    await expect.element(tileBoundaries).not.toBeChecked();

    await trafficHitAreas.click();

    await expect.element(trafficHitAreas).toBeChecked();
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
          "label": "GPS state",
          "value": "Unavailable",
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
          "label": "True airspeed state",
          "value": "Unavailable",
        },
        {
          "label": "Pressure altitude",
          "value": "–",
        },
        {
          "label": "Pressure altitude state",
          "value": "Unavailable",
        },
        {
          "label": "Vertical speed",
          "value": "–",
        },
        {
          "label": "Rate of climb",
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
          "label": "Wind ±",
          "value": "–",
        },
        {
          "label": "Fused altitude",
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
      air: null,
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
          "label": "GPS state",
          "value": "Current",
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
          "label": "True airspeed state",
          "value": "Current",
        },
        {
          "label": "Pressure altitude",
          "value": "1000 m",
        },
        {
          "label": "Pressure altitude state",
          "value": "Current",
        },
        {
          "label": "Vertical speed",
          "value": "–",
        },
        {
          "label": "Rate of climb",
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
          "label": "Wind ±",
          "value": "–",
        },
        {
          "label": "Fused altitude",
          "value": "–",
        },
      ]
    `);
  });

  it('shows every estimate the core derived', async () => {
    let instruments: Instruments = {
      ...EMPTY_INSTRUMENTS,
      air: {
        ...EMPTY_AIR_ESTIMATE,
        verticalSpeedMetersPerSecond: 1.8,
        nettoMetersPerSecond: 2.4,
        bankAngleDegrees: -38,
        windDirectionDegrees: 240,
        windSpeedMetersPerSecond: 5.2,
      },
    };
    render(MapDebugOverlay, { map: undefined, instruments, units: metricUnits });

    await userEvent.keyboard('d');

    await expect.element(page.getByText('1.80 m/s', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('2.40 m/s', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('-38°', { exact: true })).toBeInTheDocument();
    await expect.element(page.getByText('240° / 18.7 km/h', { exact: true })).toBeInTheDocument();
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
      air: null,
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
          "label": "GPS state",
          "value": "Stale",
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
          "label": "True airspeed state",
          "value": "Stale",
        },
        {
          "label": "Pressure altitude",
          "value": "3281 ft",
        },
        {
          "label": "Pressure altitude state",
          "value": "Stale",
        },
        {
          "label": "Vertical speed",
          "value": "–",
        },
        {
          "label": "Rate of climb",
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
          "label": "Wind ±",
          "value": "–",
        },
        {
          "label": "Fused altitude",
          "value": "–",
        },
      ]
    `);
  });
});
