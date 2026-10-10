import type { Page } from '@playwright/test';
import type { GeoJSONSource } from 'maplibre-gl';
import type { TrailSample } from '#lib/protocol/generated/TrailSample';

import { expect } from '@playwright/test';

import { test } from './app';

const RECORDING_START = 1_767_268_800_000;

function sample(seconds: number): TrailSample {
  return {
    unixMilliseconds: RECORDING_START + seconds * 1_000,
    position: { latitudeDegrees: 50.823 + seconds / 1_000, longitudeDegrees: 6.186 },
    altitudeMslMeters: 1_000,
    altitudeAglMeters: 800,
    varioMetersPerSecond: 1,
    nettoMetersPerSecond: 1,
    relativeVarioMetersPerSecond: 1,
  };
}

test('restores the trail after a reload and continues it with topic samples', async ({
  page,
  app,
}) => {
  await app.open('/');
  let recording = [0, 1, 2].map(sample);
  for (let value of recording) {
    await app.emit({ topic: 'trail', value: { recordingStart: RECORDING_START, sample: value } });
  }
  await expect.poll(() => readTrailIds(page)).toEqual([1, 2]);

  await page.reload();
  await page.waitForFunction(() => '__updraftFake' in window);
  await app.setFlightRecording(recording);
  await app.emit({ topic: 'trail', value: { recordingStart: RECORDING_START, sample: sample(2) } });
  await expect.poll(() => readTrailIds(page)).toEqual([1, 2]);

  await app.emit({ topic: 'trail', value: { recordingStart: RECORDING_START, sample: sample(3) } });
  await expect.poll(() => readTrailIds(page)).toEqual([1, 2, 3]);
});

/** Returns the trail segment ids in seconds after the recording start. */
async function readTrailIds(page: Page): Promise<number[] | null> {
  let ids = await page.evaluate(async () => {
    let source = window.__updraftApp?.mapState.map?.getSource<GeoJSONSource>('trail');
    let data = await source?.getData();
    return data?.type === 'FeatureCollection' ? data.features.map(({ id }) => Number(id)) : null;
  });
  return ids && ids.map((id) => (id - RECORDING_START) / 1_000);
}
