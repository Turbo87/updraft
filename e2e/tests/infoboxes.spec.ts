import type { AppContext } from '$lib/app-context';
import type { FakeClient } from '$lib/client/fake';

import { expect, test } from '@playwright/test';

type TestWindow = Window & { __updraftApp?: AppContext; __updraftFake?: FakeClient };

test('updates infoboxes from instruments, units and map zoom', async ({ page }) => {
  await page.goto('/?testMode=1');
  let dock = page.getByRole('region', { name: 'Flight instruments' });
  await expect(dock.getByRole('group')).toHaveCount(10);
  await expect(dock.getByRole('group', { name: 'Altitude', exact: true })).toContainText('–');
  await page.evaluate(() => {
    let { __updraftApp: app, __updraftFake: fake } = window as TestWindow;
    fake!.emit({
      topic: 'instruments',
      value: {
        ...app!.instruments.current,
        derived: {
          rawVerticalSpeed: null,
          verticalSpeed: null,
          vario: null,
          averageVario: null,
          wind: null,
          airspeed: null,
          heading: null,
          bank: null,
          netto: null,
          relativeVario: null,
          altitude: { altitudeMslMeters: 3048, stale: false },
        },
      },
    });
    app!.mapState.map!.jumpTo({ zoom: 11.25 });
  });
  await expect(dock.getByRole('group', { name: 'Altitude', exact: true })).toContainText('3048');
  await expect(dock.getByRole('group', { name: 'Zoom', exact: true })).toContainText('11.25');
  await page.evaluate(() => {
    let { __updraftApp: app, __updraftFake: fake } = window as TestWindow;
    fake!.emit({
      topic: 'settings',
      value: {
        ...app!.settings.current,
        units: { ...app!.settings.current.units, altitude: 'ft' },
      },
    });
  });
  await expect(dock.getByRole('group', { name: 'Altitude', exact: true })).toContainText(
    '10000 ft',
  );
});

for (let scenario of [
  { name: 'portrait', width: 360, height: 780, top: 32, right: 0, bottom: 24, left: 0, columns: 5 },
  {
    name: 'landscape left cutout',
    width: 780,
    height: 360,
    top: 24,
    right: 0,
    bottom: 20,
    left: 32,
    columns: 2,
  },
  {
    name: 'landscape right cutout',
    width: 780,
    height: 360,
    top: 24,
    right: 32,
    bottom: 20,
    left: 0,
    columns: 2,
  },
]) {
  for (let locale of ['en', 'de'] as const) {
    test(`keeps infoboxes inside the safe area in ${scenario.name} (${locale})`, async ({
      page,
    }) => {
      await page.setViewportSize({ width: scenario.width, height: scenario.height });
      await page.goto('/?testMode=1');
      await page.waitForFunction(() => '__updraftFake' in window);
      await page.evaluate((locale) => {
        let { __updraftApp: app, __updraftFake: fake } = window as TestWindow;
        fake!.emit({ topic: 'settings', value: { ...app!.settings.current, locale } });
      }, locale);
      await page.locator('html').evaluate((root, insets) => {
        for (let edge of ['top', 'right', 'bottom', 'left'] as const)
          root.style.setProperty(`--safe-area-${edge}`, `${insets[edge]}px`);
      }, scenario);
      let dock = page.getByRole('region', {
        name: locale === 'en' ? 'Flight instruments' : 'Fluginstrumente',
      });
      await expect(dock.getByRole('group')).toHaveCount(10);
      for (let edge of ['top', 'right', 'bottom', 'left']) {
        await expect(dock.locator('.cells')).toHaveCSS(`border-${edge}-width`, '1px');
        await expect(dock.locator('.cells')).toHaveCSS(`border-${edge}-style`, 'solid');
      }
      await page.evaluate(async () => {
        await document.fonts.ready;
      });
      expect(
        await dock
          .locator('.label')
          .evaluateAll((labels) =>
            labels.every((label) => label.scrollHeight <= label.clientHeight),
          ),
      ).toBe(true);
      if (locale === 'de') {
        expect(
          await dock
            .locator('.label')
            .evaluateAll((labels) =>
              labels.every(
                (label) =>
                  label.clientHeight <= parseFloat(getComputedStyle(label).lineHeight) &&
                  label.scrollWidth <= label.clientWidth,
              ),
            ),
        ).toBe(true);
      }
      let cells = await dock.getByRole('group').evaluateAll((elements) =>
        elements.map((element) => {
          let { x, y, width, height } = element.getBoundingClientRect();
          return { x, y, width, height };
        }),
      );
      for (let cell of cells) {
        expect(cell.x).toBeGreaterThanOrEqual(scenario.left);
        expect(cell.y).toBeGreaterThanOrEqual(scenario.top);
        expect(cell.x + cell.width).toBeLessThanOrEqual(scenario.width - scenario.right);
        expect(cell.y + cell.height).toBeLessThanOrEqual(scenario.height - scenario.bottom);
      }
      expect(cells[scenario.columns].y).toBeGreaterThan(cells[0].y);
      expect(cells[scenario.columns].x).toBe(cells[0].x);
      let bounds = await dock.boundingBox();
      expect(bounds!.x + bounds!.width).toBe(scenario.width);
      expect(bounds!.y + bounds!.height).toBe(scenario.height);
      let map = page.locator('.maplibregl-map');
      let mapBounds = await map.boundingBox();
      if (scenario.columns === 5) expect(mapBounds!.y + mapBounds!.height).toBe(bounds!.y);
      else expect(mapBounds!.x + mapBounds!.width).toBe(bounds!.x);
      await page.setViewportSize({ width: scenario.height, height: scenario.width });
      await expect
        .poll(async () =>
          map.evaluate((element) => {
            let canvas = element.querySelector('canvas')!;
            return (
              canvas.clientWidth === element.clientWidth &&
              canvas.clientHeight === element.clientHeight
            );
          }),
        )
        .toBe(true);
    });
  }
}
