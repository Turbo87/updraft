import type { Page } from '@playwright/test';
import type * as GeoJSON from 'geojson';
import type { GeoJSONSource, GeoJSONSourceSpecification } from 'maplibre-gl';
import type { AirspaceProperties } from '$lib/airspace';
import type { AppContext } from '$lib/app-context';
import type { GpsInstruments } from '$lib/protocol/generated/GpsInstruments';
import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
import type { TrafficUpdate } from '$lib/protocol/generated/TrafficUpdate';

import { expect, test } from '@playwright/test';

import { AIRSPACE_BROWSER_FIXTURE } from '../../frontend/src/lib/map/airspace.fixture';

type MapState = {
  center: number[];
  renderedCoordinates: number[] | null;
  sourceCoordinates: number[];
};

type MapCenter = {
  latitudeDegrees: number;
  longitudeDegrees: number;
};

type AirspaceMapState = {
  featureCount: number;
  layerOrder: string[];
  renderedLayerIds: string[];
};

type TestWindow = Window & {
  __updraftApp?: AppContext;
  __updraftFake?: { emit: (topic: unknown) => void };
  __updraftTestAirspaceData?: GeoJSONSourceSpecification['data'];
};

const POSITION_A: GpsInstruments = {
  position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
  trackDegrees: 45,
  groundSpeedMetersPerSecond: 30,
  altitudeMeters: 400,
  fixTime: null,
  stale: false,
};

const POSITION_B: GpsInstruments = {
  position: { latitudeDegrees: 50.824, longitudeDegrees: 6.187 },
  trackDegrees: 90,
  groundSpeedMetersPerSecond: 31,
  altitudeMeters: 410,
  fixTime: null,
  stale: false,
};

const POSITION_C: GpsInstruments = {
  position: { latitudeDegrees: 50.825, longitudeDegrees: 6.188 },
  trackDegrees: 135,
  groundSpeedMetersPerSecond: 32,
  altitudeMeters: 420,
  fixTime: null,
  stale: false,
};

const TRAFFIC_A: PublishedTrafficTarget = {
  id: 'flarm:000001',
  position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
  altitudeMslMeters: 400,
  trafficType: 'glider',
  trackDegrees: 45,
  alarmLevel: 'none',
  stale: false,
};

const TRAFFIC_B: PublishedTrafficTarget = {
  id: 'flarm:000002',
  position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
  altitudeMslMeters: 500,
  trafficType: 'towPlane',
  trackDegrees: 90,
  alarmLevel: 'none',
  stale: false,
};

test('follows live positions until the user pans and returns', async ({ page }) => {
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);

  await emitInstruments(page, POSITION_A);
  await expectMapPosition(page, POSITION_A);

  let followedCenter = await readMapCenter(page);
  await panMap(page);
  let returnButton = page.getByRole('button', { name: 'Return to position' });
  await expect(returnButton).toBeVisible();
  let manualCenter = await expectFullPan(page, followedCenter);

  await panMap(page);
  manualCenter = await expectFullPan(page, manualCenter);

  await emitInstruments(page, POSITION_B);
  await expectMapPosition(page, POSITION_B, manualCenter);

  await returnButton.click();
  await expect(returnButton).not.toBeVisible();
  await expectMapPosition(page, POSITION_B);

  await emitInstruments(page, POSITION_C);
  await expectMapPosition(page, POSITION_C);
});

test('keeps the Settings button inside the safe area', async ({ page }) => {
  await page.goto('/?testMode=1');
  await page.locator('html').evaluate((element) => {
    element.style.setProperty('--safe-area-top', '32px');
    element.style.setProperty('--safe-area-right', '24px');
  });

  await expect(page.locator('meta[name="viewport"]')).toHaveAttribute(
    'content',
    'width=device-width, initial-scale=1, viewport-fit=cover',
  );

  let settingsButton = page.getByRole('link', { name: 'Settings' });
  await expect(settingsButton).toHaveCSS('width', '56px');
  await expect(settingsButton).toHaveCSS('height', '56px');
  await expect(settingsButton.locator('..')).toHaveCSS('top', '48px');
  await expect(settingsButton.locator('..')).toHaveCSS('right', '40px');
});

test('keeps the return-to-position button inside the safe area', async ({ page }) => {
  await page.goto('/?testMode=1');
  await page.locator('html').evaluate((element) => {
    element.style.setProperty('--safe-area-right', '24px');
    element.style.setProperty('--safe-area-bottom', '20px');
  });

  await panMap(page);
  let returnButton = page.getByRole('button', { name: 'Return to position' });
  await expect(returnButton).toHaveCSS('width', '56px');
  await expect(returnButton).toHaveCSS('height', '56px');
  await expect(returnButton.locator('..')).toHaveCSS('right', '40px');
  await expect(returnButton.locator('..')).toHaveCSS('bottom', '36px');
});

test('renders active airspace below traffic and ownship', async ({ page }) => {
  await page.addInitScript((data) => {
    (window as TestWindow).__updraftTestAirspaceData = data;
  }, AIRSPACE_BROWSER_FIXTURE);
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);

  await emitInstruments(page, POSITION_A);
  await page.evaluate(() => {
    (window as TestWindow).__updraftFake?.emit({
      topic: 'airspace',
      value: {
        generation: 1,
        sources: [{ type: 'active', sourceName: 'browser-fixture.txt', airspaceCount: 2 }],
      },
    });
  });

  await expect
    .poll(() => readAirspaceMapState(page), { message: 'map to render the airspace layers' })
    .toEqual({
      featureCount: 2,
      layerOrder: [
        'airspace-inner-band',
        'airspace-outline',
        'traffic-fixed',
        'traffic-directional',
        'ownship-symbol',
      ],
      renderedLayerIds: ['airspace-inner-band', 'airspace-outline'],
    });
});

test('opens a tapped map position and updates its ownship relation', async ({ page }) => {
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);

  await emitInstruments(page, POSITION_A);
  await expectMapPosition(page, POSITION_A);

  let bounds = await page.locator('.maplibregl-canvas').boundingBox();
  if (!bounds) throw new Error('Map canvas is not visible');

  await page.mouse.click(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
  await expect(page).toHaveURL('/nearby/50.823000/6.186000');
  await expect(page.getByRole('heading', { name: 'Nearby' })).toBeVisible();
  await expect(page.getByText('50.82300° N, 6.18600° E')).toBeVisible();
  await expect(page.getByText('0.0', { exact: true })).toBeVisible();
  await expect(page.getByText('km', { exact: true })).toBeVisible();
  await expect(page.getByText('0', { exact: true })).toBeVisible();
  await expect(page.getByText('°', { exact: true })).toBeVisible();
  await expect(page.getByText('Arrival')).toBeVisible();
  await expect(page.getByText('Req. L/D')).toBeVisible();
  await expect(page.getByText('Elevation')).toBeVisible();
  await expect(page.getByText('—', { exact: true })).toHaveCount(3);

  await emitInstruments(page, POSITION_B);
  await expect(page.getByText('0.1', { exact: true })).toBeVisible();
  await expect(page.getByText('212', { exact: true })).toBeVisible();

  await page.getByRole('link', { name: 'Back to map' }).click();
  await expect(page).toHaveURL('/');

  await page.goto('/nearby/91/6.186?testMode=1');
  await expect(page.getByRole('heading', { name: 'Nearby' })).toBeVisible();
  await expect(page.getByText('The selected map position is invalid.')).toBeVisible();
  await expect(page.getByRole('main')).not.toContainText('Back to map');
  await page.getByRole('link', { name: 'Back to map' }).click();
  await expect(page).toHaveURL('/');
});

test('shows overlapping nearby airspaces in MapLibre order', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 780 });
  let airspaceFixture = structuredClone(AIRSPACE_BROWSER_FIXTURE);
  airspaceFixture.features[1].properties.name = 'SIV MARSEILLE NORD 1 EXTENDED AIRSPACE';
  airspaceFixture.features[1].properties.type = 29;
  await page.addInitScript((data) => {
    (window as TestWindow).__updraftTestAirspaceData = data;
  }, airspaceFixture);
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitAirspace(page, { type: 'active', generation: 1 });
  await expect.poll(() => readAirspaceMapState(page)).not.toBeNull();

  await clickMapPosition(page, { latitudeDegrees: 50.82, longitudeDegrees: 6.182 });
  let airspaces = page.getByRole('region', { name: 'Airspaces' });
  await expect(airspaces.getByRole('listitem')).toHaveText([
    /SIV MARSEILLE NORD 1 EXTENDED AIRSPACE\s+Low-altitude overflight restriction · Class E/,
    /Düsseldorf CTR\s+Control zone · Class D/,
  ]);
  let firstRow = airspaces.getByRole('listitem').first();
  let name = airspaces.getByText('SIV MARSEILLE NORD 1 EXTENDED AIRSPACE');
  let detail = airspaces.getByText(/Low-altitude overflight restriction/);
  let nameBox = await name.boundingBox();
  let detailBox = await detail.boundingBox();
  if (!nameBox || !detailBox) throw new Error('Airspace row text is not visible');
  expect(detailBox.y).toBeGreaterThanOrEqual(nameBox.y + nameBox.height);
  for (let value of [name, detail]) {
    await expect
      .poll(() =>
        value.evaluate((element) => {
          let style = getComputedStyle(element);
          return [style.overflow, style.textOverflow, style.whiteSpace];
        }),
      )
      .toEqual(['hidden', 'ellipsis', 'nowrap']);
  }
  let rowBox = await firstRow.boundingBox();
  let chevronBox = await firstRow.locator('.chevron').boundingBox();
  if (!rowBox || !chevronBox) throw new Error('Airspace row is not visible');
  expect(chevronBox.x + chevronBox.width).toBeLessThanOrEqual(rowBox.x + rowBox.width);
  await expect(firstRow.getByRole('link')).toHaveCSS('border-width', '0px');
  let scrollingRegion = page.getByRole('main');
  expect(await scrollingRegion.evaluate((element) => element.scrollWidth)).toBe(
    await scrollingRegion.evaluate((element) => element.clientWidth),
  );
});

test('shows empty states without rendered features', async ({ page }) => {
  await page.goto('/nearby/50.82/6.15?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await expect(
    page.getByText('No GPS position is available. Values relative to ownship are unknown.'),
  ).toBeVisible();
  let airspaces = page.getByRole('region', { name: 'Airspaces' });
  await expect(airspaces.getByText('No airspace at this position.')).toBeVisible();
  let traffic = page.getByRole('region', { name: 'Traffic' });
  await expect(traffic.getByText('No traffic at this position.')).toBeVisible();
  let waypointEmpty = page.getByText('No nearby waypoints.');
  await expect(waypointEmpty).toBeVisible();
  for (let property of [
    'padding',
    'border',
    'border-radius',
    'background-color',
    'color',
    'font',
  ]) {
    let expected = await traffic
      .getByText('No traffic at this position.')
      .evaluate(
        (element, property) => getComputedStyle(element).getPropertyValue(property),
        property,
      );
    await expect(waypointEmpty).toHaveCSS(property, expected);
  }

  await page.getByRole('link', { name: 'Back to map' }).click();
  await expect(page).toHaveURL('/');
  await emitAirspace(page, { type: 'unavailable' });
  await clickMapPosition(page, { latitudeDegrees: 50.82, longitudeDegrees: 6.15 });
  await expect(airspaces.getByText('No airspace at this position.')).toBeVisible();
});

test('invalidates nearby airspaces when the catalog changes', async ({ page }) => {
  await page.addInitScript((data) => {
    (window as TestWindow).__updraftTestAirspaceData = data;
  }, AIRSPACE_BROWSER_FIXTURE);
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitAirspace(page, { type: 'active', generation: 1 });
  await expect.poll(() => readAirspaceMapState(page)).not.toBeNull();

  await clickMapPosition(page, { latitudeDegrees: 50.82, longitudeDegrees: 6.19 });
  let airspaces = page.getByRole('region', { name: 'Airspaces' });
  await expect(airspaces.getByRole('listitem')).toHaveText(
    /Köln RMZ\s+Radio mandatory zone · Class E/,
  );
  await expect(airspaces.getByRole('link', { name: 'Köln RMZ' })).toHaveAttribute(
    'href',
    '/airspaces/1:0:1',
  );

  await emitAirspace(page, { type: 'unavailable' });
  await expect(airspaces.getByText('No airspace at this position.')).toBeVisible();
});

test('keeps nearby traffic membership while targets update', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 780 });
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitInstruments(page, POSITION_A);
  await emitTraffic(page, { type: 'snapshot', value: [TRAFFIC_A, TRAFFIC_B] });
  let selectedPosition = TRAFFIC_A.position;
  await expect
    .poll(() => readTrafficIdsAt(page, selectedPosition))
    .toEqual([TRAFFIC_B.id, TRAFFIC_A.id]);

  await clickMapPosition(page, selectedPosition);
  let traffic = page.getByRole('region', { name: 'Traffic' });
  await expect(traffic.getByRole('listitem')).toHaveText([
    /Tow plane · FLARM 000002\s+500 m MSL · \+100 m · 0\.0 km/,
    /Glider · FLARM 000001\s+400 m MSL · 0 m · 0\.0 km/,
  ]);
  let firstTrafficRow = traffic.getByRole('listitem').first();
  let trafficLabel = firstTrafficRow.getByText('Tow plane · FLARM 000002');
  let trafficDetail = firstTrafficRow.getByText('500 m MSL · +100 m · 0.0 km');
  let trafficLabelBox = await trafficLabel.boundingBox();
  let trafficDetailBox = await trafficDetail.boundingBox();
  if (!trafficLabelBox || !trafficDetailBox) throw new Error('Traffic row text is not visible');
  expect(trafficDetailBox.y).toBeGreaterThanOrEqual(trafficLabelBox.y + trafficLabelBox.height);
  await expect(firstTrafficRow.locator('.traffic-symbol')).toHaveCSS('width', '32px');

  let updated = {
    ...TRAFFIC_A,
    position: { latitudeDegrees: 0, longitudeDegrees: 0 },
    trafficType: 'balloon' as const,
  };
  let unrelated = { ...TRAFFIC_A, id: 'flarm:000003' };
  await emitTraffic(page, {
    type: 'delta',
    value: { upserts: [updated, unrelated], removed: [TRAFFIC_B.id] },
  });
  await expect(traffic.getByRole('listitem')).toHaveText([
    /Tow plane · FLARM 000002 · Unavailable\s+500 m MSL · \+100 m · 0\.0 km/,
    /Balloon · FLARM 000001/,
  ]);

  let recovered = { ...TRAFFIC_B, trafficType: 'paraglider' as const };
  await emitTraffic(page, {
    type: 'delta',
    value: { upserts: [recovered], removed: [] },
  });
  await expect(traffic.getByRole('listitem')).toHaveText([
    /Paraglider · FLARM 000002\s+500 m MSL · \+100 m · 0\.0 km/,
    /Balloon · FLARM 000001/,
  ]);

  await emitTraffic(page, {
    type: 'delta',
    value: { upserts: [], removed: [TRAFFIC_B.id] },
  });
  await traffic.getByRole('link', { name: /Paraglider · FLARM 000002 · Unavailable/ }).click();
  await expect(page).toHaveURL('/traffic/flarm:000002');
  await expect(page.getByText('Traffic not found.')).toBeVisible();
});

test('does not move the map for an off-screen nearby URL', async ({ page }) => {
  await page.addInitScript((data) => {
    (window as TestWindow).__updraftTestAirspaceData = data;
  }, AIRSPACE_BROWSER_FIXTURE);
  await page.goto('/nearby/0/0?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await expect.poll(() => readMapCenter(page)).toBeTruthy();
  let initialCenter = await readMapCenter(page);

  await emitAirspace(page, { type: 'active', generation: 1 });
  let airspaces = page.getByRole('region', { name: 'Airspaces' });
  await expect(airspaces.getByText('No airspace at this position.')).toBeVisible();
  expect(await readMapCenter(page)).toEqual(initialCenter);
});

test('shows complete airspace details on direct visits and reloads', async ({ page }) => {
  await page.addInitScript((data) => {
    (window as TestWindow).__updraftTestAirspaceData = data;
  }, AIRSPACE_BROWSER_FIXTURE);
  await page.goto('/airspaces/1:0:0?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitAirspace(page, { type: 'active', generation: 1 });

  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Düsseldorf CTR');
  await expect(page.getByRole('button', { name: 'Back' })).toBeVisible();
  await expect(page.locator('main > div')).toMatchAriaSnapshot(`
    - text: Upper 1524 m MSL Lower GND Class D
    - heading "Classification" [level=2]
    - term: Type
    - definition: Control zone
    - term: ICAO class
    - definition: Class D
    - term: Activity
    - definition: Hang gliding or paragliding
    - heading "Vertical limits" [level=2]
    - term: Upper limit
    - definition: 1524 m MSL (max. FL 120)
    - term: Lower limit
    - definition: GND (min. 152 m AGL)
    - heading "Countries" [level=2]
    - list:
      - listitem: DE
      - listitem: AT
    - heading "Communications" [level=2]
    - term: TOWER
    - definition: 123.450 MHz
    - term: Primary
    - definition: "Yes"
    - term: Remarks
    - definition: EMERGENCIES ONLY
    - term: Frequency
    - definition: 121.500 MHz
    - term: Remarks
    - definition: GUARD
    - term: Transponder code
    - definition: "0123"
    - term: Primary
    - definition: "Yes"
    - term: Remarks
    - definition: WHEN ACTIVE
    - heading "Activation" [level=2]
    - term: On demand
    - definition: "Yes"
    - term: On request
    - definition: "No"
    - term: By NOTAM
    - definition: "Yes"
    - term: Special agreement
    - definition: "No"
    - term: Compliance requested
    - definition: "Yes"
    - term: Active from
    - definition: Apr 12, 2026, 8:30 AM
    - term: Active until
    - definition: Apr 12, 2026, 5:45 PM
    - heading "Operating hours" [level=3]
    - heading "Sunday" [level=4]
    - term: Start
    - definition: Sunrise
    - term: End
    - definition: Sunset
    - term: By NOTAM
    - definition: "No"
    - term: Public holidays excluded
    - definition: "Yes"
    - term: Remarks
    - definition: DAYLIGHT HOURS
    - term: Remarks
    - definition: LOCAL TIME
    - heading "Remarks" [level=2]
    - paragraph: ACTIVE DURING GLIDER EVENTS
  `);

  await page.reload();
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitAirspace(page, { type: 'active', generation: 2 });
  await expect(page.getByText('Airspace not found.')).toBeVisible();
});

test('shows airspace not found for unavailable data and missing IDs', async ({ page }) => {
  await page.addInitScript((data) => {
    (window as TestWindow).__updraftTestAirspaceData = data;
  }, AIRSPACE_BROWSER_FIXTURE);
  await page.goto('/airspaces/1:0:0?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitAirspace(page, { type: 'unavailable' });
  await expect(page.getByText('Airspace not found.')).toBeVisible();

  await page.goto('/airspaces/1:0:999?testMode=1');
  await emitAirspace(page, { type: 'active', generation: 1 });
  await expect(page.getByText('Airspace not found.')).toBeVisible();
});

test('omits an unclassified airspace class', async ({ page }) => {
  let fixture = structuredClone(AIRSPACE_BROWSER_FIXTURE) as GeoJSON.FeatureCollection<
    GeoJSON.Polygon,
    AirspaceProperties
  >;
  fixture.features[0].properties.icaoClass = 8;
  await page.addInitScript((data) => {
    (window as TestWindow).__updraftTestAirspaceData = data;
  }, fixture);
  await page.goto('/airspaces/1:0:0?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitAirspace(page, { type: 'active', generation: 1 });

  let classification = page.getByRole('heading', { name: 'Classification' }).locator('..');
  await expect(classification.getByText('ICAO class')).toHaveCount(0);
  await expect(classification.getByText('Unclassified')).toHaveCount(0);
});

test('retries an airspace source read failure', async ({ page }) => {
  await page.addInitScript(() => {
    (window as TestWindow).__updraftTestAirspaceData = '/missing-airspace.geojson';
  });
  await page.goto('/airspaces/1:0:0?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitAirspace(page, { type: 'active', generation: 1 });

  let retry = page.getByRole('button', { name: 'Retry' });
  await expect(page.getByText('The airspace could not be loaded.')).toBeVisible();
  await expect(retry).toBeVisible();

  await page.evaluate(async (data) => {
    let source = (window as TestWindow).__updraftApp?.mapState.map?.getSource<GeoJSONSource>(
      'airspace',
    );
    if (!source) throw new Error('Airspace source is not available');
    await source.setData(data);
  }, AIRSPACE_BROWSER_FIXTURE);
  await retry.click();
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Düsseldorf CTR');
});

test('shows complete traffic details on direct visits and reloads', async ({ page }) => {
  await page.goto(`/traffic/${TRAFFIC_A.id}?testMode=1`);
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitInstruments(page, POSITION_A);
  await emitTraffic(page, { type: 'snapshot', value: [TRAFFIC_A] });

  await expect(page.getByRole('heading', { level: 1 })).toHaveText('FLARM 000001');
  await expect(page.getByRole('button', { name: 'Back' })).toBeVisible();
  await expect(page.locator('main')).toMatchAriaSnapshot(`
    - main:
      - text: Distance 0.0 km Bearing 000 ° Alarm level None
      - heading "Target" [level=2]
      - term: ID
      - definition: flarm:000001
      - term: Type
      - definition: Glider
      - term: State
      - definition: Fresh
      - heading "Position" [level=2]
      - term: Position
      - definition: 50.82300° N, 6.18600° E
      - term: Altitude
      - definition: 400 m MSL 0 m
      - term: Track
      - definition: 45° true
  `);

  await page.reload();
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitTraffic(page, { type: 'snapshot', value: [TRAFFIC_A] });
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('FLARM 000001');
});

test('updates and retains traffic details', async ({ page }) => {
  await page.goto(`/traffic/${TRAFFIC_A.id}?testMode=1`);
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitInstruments(page, POSITION_A);
  await emitTraffic(page, { type: 'snapshot', value: [TRAFFIC_A] });

  let movedTarget = {
    ...TRAFFIC_A,
    position: POSITION_B.position,
    altitudeMslMeters: null,
    trackDegrees: null,
    alarmLevel: 'important' as const,
    stale: true,
  };
  await emitTraffic(page, {
    type: 'delta',
    value: { upserts: [movedTarget], removed: [] },
  });
  await expect(page.getByText('50.82400° N, 6.18700° E')).toBeVisible();
  await expect(page.getByText('0.1 km', { exact: true })).toBeVisible();
  await expect(page.getByText('032', { exact: true })).toBeVisible();
  await expect(page.getByText('Stale', { exact: true })).toBeVisible();
  await expect(page.getByText('Important', { exact: true })).toBeVisible();
  await expect(page.getByText('—', { exact: true })).toHaveCount(3);

  await emitInstruments(page, POSITION_C);
  await expect(page.getByText('212', { exact: true })).toBeVisible();

  await emitTraffic(page, {
    type: 'delta',
    value: { upserts: [], removed: [TRAFFIC_A.id] },
  });
  await expect(page.getByText('Unavailable', { exact: true })).toBeVisible();
  await expect(page.getByText('50.82400° N, 6.18700° E')).toBeVisible();

  await emitTraffic(page, {
    type: 'delta',
    value: { upserts: [TRAFFIC_A], removed: [] },
  });
  await expect(page.getByText('Fresh', { exact: true })).toBeVisible();
  await expect(page.getByText('50.82300° N, 6.18600° E')).toBeVisible();
});

test('shows traffic not found for missing IDs', async ({ page }) => {
  await page.goto('/traffic/flarm:999999?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitTraffic(page, { type: 'snapshot', value: [TRAFFIC_A] });
  await expect(page.getByText('Traffic not found.')).toBeVisible();
});

test('opens inspector details and requeries the map after browser Back', async ({ page }) => {
  await page.addInitScript((data) => {
    (window as TestWindow).__updraftTestAirspaceData = data;
  }, AIRSPACE_BROWSER_FIXTURE);
  await page.goto('/?testMode=1');
  await page.waitForFunction(() => '__updraftFake' in window);
  await emitAirspace(page, { type: 'active', generation: 1 });

  let selectedPosition = { latitudeDegrees: 50.82, longitudeDegrees: 6.182 };
  let trafficTarget = { ...TRAFFIC_A, position: selectedPosition };
  await emitTraffic(page, { type: 'snapshot', value: [trafficTarget] });
  await expect.poll(() => readAirspaceMapState(page)).not.toBeNull();
  await expect.poll(() => readTrafficIdsAt(page, selectedPosition)).toEqual([TRAFFIC_A.id]);

  await clickMapPosition(page, selectedPosition);
  let nearbyUrl = page.url();
  let airspaces = page.getByRole('region', { name: 'Airspaces' });
  let traffic = page.getByRole('region', { name: 'Traffic' });
  await expect(airspaces.getByRole('listitem')).toHaveText([
    /Köln RMZ\s+Radio mandatory zone · Class E/,
    /Düsseldorf CTR\s+Control zone · Class D/,
  ]);
  await expect(traffic.getByRole('listitem')).toHaveText(
    /Glider · FLARM 000001\s+400 m MSL · — · —/,
  );

  await airspaces.getByRole('link', { name: 'Köln RMZ' }).click();
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Köln RMZ');
  await page.getByRole('button', { name: 'Back' }).click();
  await expect(page).toHaveURL(nearbyUrl);
  await expect(traffic.getByRole('link', { name: /^Glider · FLARM 000001/ })).toBeVisible();

  await traffic.getByRole('link', { name: /^Glider · FLARM 000001/ }).click();
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('FLARM 000001');
  await emitTraffic(page, {
    type: 'delta',
    value: {
      upserts: [{ ...trafficTarget, position: { latitudeDegrees: 0, longitudeDegrees: 0 } }],
      removed: [],
    },
  });
  await expect.poll(() => readTrafficIdsAt(page, selectedPosition)).toEqual([]);

  await page.getByRole('button', { name: 'Back' }).click();
  await expect(page).toHaveURL(nearbyUrl);
  await expect(traffic.getByText('No traffic at this position.')).toBeVisible();
  await expect(airspaces.getByRole('listitem')).toHaveText([
    /Köln RMZ\s+Radio mandatory zone · Class E/,
    /Düsseldorf CTR\s+Control zone · Class D/,
  ]);
});

async function emitInstruments(page: Page, gps: GpsInstruments) {
  await page.evaluate(
    (value) => {
      (window as TestWindow).__updraftFake?.emit({ topic: 'instruments', value });
    },
    { gps, pressureAltitude: null },
  );
}

async function emitAirspace(
  page: Page,
  state: { type: 'none' } | { type: 'unavailable' } | { type: 'active'; generation: number },
) {
  await page.evaluate(
    ({ airspace, airspaceCount }) => {
      let value = {
        generation: airspace.type === 'active' ? airspace.generation : 0,
        sources:
          airspace.type === 'active'
            ? [{ type: 'active' as const, sourceName: 'browser-fixture.txt', airspaceCount }]
            : airspace.type === 'unavailable'
              ? [
                  {
                    type: 'unavailable' as const,
                    sourceName: 'broken.txt',
                    error: 'readFailed' as const,
                  },
                ]
              : [],
      };

      (window as TestWindow).__updraftFake?.emit({ topic: 'airspace', value });
    },
    { airspace: state, airspaceCount: AIRSPACE_BROWSER_FIXTURE.features.length },
  );
}

async function emitTraffic(page: Page, update: TrafficUpdate) {
  await page.evaluate((value) => {
    (window as TestWindow).__updraftFake?.emit({ topic: 'traffic', value });
  }, update);
}

async function clickMapPosition(page: Page, position: MapCenter) {
  let point = await page.evaluate(({ latitudeDegrees, longitudeDegrees }) => {
    let map = (window as TestWindow).__updraftApp?.mapState.map;
    if (!map) throw new Error('Map is not available');
    return map.project([longitudeDegrees, latitudeDegrees]);
  }, position);
  let bounds = await page.locator('.maplibregl-canvas').boundingBox();
  if (!bounds) throw new Error('Map canvas is not visible');

  await page.mouse.click(bounds.x + point.x, bounds.y + point.y);
  await expect(page).toHaveURL(/\/nearby\//);
}

async function expectMapPosition(
  page: Page,
  instruments: GpsInstruments,
  expectedCenter = instruments.position,
) {
  let { latitudeDegrees, longitudeDegrees } = instruments.position;

  await expect
    .poll(() => readMapState(page), {
      message: `map to render position ${latitudeDegrees}, ${longitudeDegrees}`,
    })
    .toEqual({
      center: [
        expect.closeTo(expectedCenter.longitudeDegrees, 6),
        expect.closeTo(expectedCenter.latitudeDegrees, 6),
      ],
      renderedCoordinates: [
        expect.closeTo(longitudeDegrees, 4),
        expect.closeTo(latitudeDegrees, 4),
      ],
      sourceCoordinates: [expect.closeTo(longitudeDegrees, 6), expect.closeTo(latitudeDegrees, 6)],
    });
}

async function panMap(page: Page) {
  let bounds = await page.locator('.maplibregl-canvas').boundingBox();
  if (!bounds) throw new Error('Map canvas is not visible');

  let centerX = bounds.x + bounds.width / 2;
  let centerY = bounds.y + bounds.height / 2;
  await page.mouse.move(centerX, centerY);
  await page.mouse.down();
  await page.mouse.move(centerX + 120, centerY, { steps: 5 });
  await page.mouse.up();
}

async function readMapCenter(page: Page): Promise<MapCenter> {
  return page.evaluate(() => {
    let map = (window as TestWindow).__updraftApp?.mapState.map;
    if (!map) throw new Error('Map is not available');

    let center = map.getCenter();
    return { latitudeDegrees: center.lat, longitudeDegrees: center.lng };
  });
}

async function expectFullPan(page: Page, previousCenter: MapCenter): Promise<MapCenter> {
  await expect
    .poll(() => page.evaluate(() => (window as TestWindow).__updraftApp?.mapState.map?.isMoving()))
    .toBe(false);

  let { center, displacementPixels } = await page.evaluate((previous) => {
    let map = (window as TestWindow).__updraftApp?.mapState.map;
    if (!map) throw new Error('Map is not available');

    let current = map.getCenter();
    let previousPoint = map.project([previous.longitudeDegrees, previous.latitudeDegrees]);
    let currentPoint = map.project(current);
    return {
      center: { latitudeDegrees: current.lat, longitudeDegrees: current.lng },
      displacementPixels: Math.abs(previousPoint.x - currentPoint.x),
    };
  }, previousCenter);
  expect(displacementPixels).toBeGreaterThan(100);
  return center;
}

async function readMapState(page: Page): Promise<MapState | null> {
  return page.evaluate(async () => {
    let map = (window as TestWindow).__updraftApp?.mapState.map;
    let source = map?.getSource<GeoJSONSource>('ownship');
    if (!map || !source) return null;

    let data = await source.getData();
    if (data.type !== 'Feature' || data.geometry?.type !== 'Point') return null;

    let center = map.getCenter();
    let renderedOwnship = map.queryRenderedFeatures({ layers: ['ownship-symbol'] })[0];

    return {
      center: [center.lng, center.lat],
      renderedCoordinates:
        renderedOwnship?.geometry.type === 'Point' ? renderedOwnship.geometry.coordinates : null,
      sourceCoordinates: data.geometry.coordinates,
    };
  });
}

async function readAirspaceMapState(page: Page): Promise<AirspaceMapState | null> {
  return page.evaluate(async () => {
    let map = (window as TestWindow).__updraftApp?.mapState.map;
    let source = map?.getSource<GeoJSONSource>('airspace');
    if (!map || !source || !map.isSourceLoaded('airspace')) return null;

    let data = await source.getData();
    if (data.type !== 'FeatureCollection') return null;

    let airspaceLayerIds = ['airspace-inner-band', 'airspace-outline'];
    if (airspaceLayerIds.some((id) => !map.getLayer(id))) return null;

    let relevantLayerIds = new Set([
      ...airspaceLayerIds,
      'traffic-fixed',
      'traffic-directional',
      'ownship-symbol',
    ]);
    let layerOrder = (map.getStyle().layers ?? [])
      .map((layer) => layer.id)
      .filter((id) => relevantLayerIds.has(id));
    let renderedLayerIds = [
      ...new Set(
        map.queryRenderedFeatures({ layers: airspaceLayerIds }).map((feature) => feature.layer.id),
      ),
    ].sort();

    return {
      featureCount: data.features.length,
      layerOrder,
      renderedLayerIds,
    };
  });
}

async function readTrafficIdsAt(page: Page, position: MapCenter): Promise<(string | number)[]> {
  return page.evaluate(({ latitudeDegrees, longitudeDegrees }) => {
    let map = (window as TestWindow).__updraftApp?.mapState.map;
    if (!map?.getLayer('traffic-hit')) return [];

    let point = map.project([longitudeDegrees, latitudeDegrees]);
    return map
      .queryRenderedFeatures(point, { layers: ['traffic-hit'] })
      .flatMap(({ id }) => (id === undefined ? [] : [id]));
  }, position);
}
