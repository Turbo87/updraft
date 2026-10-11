import type { DerivedWindInstruments } from '#lib/protocol/generated/DerivedWindInstruments.js';
import type { Trail } from '#lib/protocol/generated/Trail.js';
import type { TrailSample } from '#lib/protocol/generated/TrailSample.js';
import type { TrailSubscriber } from './trail.svelte';

import { afterEach, describe, expect, it, vi } from 'vitest';

import { settingsFixture } from '#lib/settings.fixture.js';
import { EMPTY_DERIVED_INSTRUMENTS, EMPTY_INSTRUMENTS } from './instruments.svelte';
import { TrailStore } from './trail.svelte';

const MINUTE = 60_000;
const WIND = { directionDegrees: 270, speedMetersPerSecond: 10, stale: false };

function sample(minutes: number): TrailSample {
  return {
    unixMilliseconds: minutes * MINUTE,
    position: { latitudeDegrees: 50 + minutes / 1000, longitudeDegrees: 6 },
    altitudeMslMeters: null,
    altitudeAglMeters: null,
    varioMetersPerSecond: null,
    verticalSpeedMetersPerSecond: null,
    nettoMetersPerSecond: null,
    relativeVarioMetersPerSecond: null,
    estimatedNettoMetersPerSecond: null,
    estimatedRelativeVarioMetersPerSecond: null,
  };
}

function trail(minutes: number, recordingStart = 0): Trail {
  return {
    recordingStart,
    sample: sample(minutes),
    maxAltitudeMslMeters: null,
    maxAltitudeAglMeters: null,
  };
}

/** Returns a store whose resource requests stay pending until the test resolves them. */
function setup() {
  let requests: { since: number; resolve: (samples: TrailSample[]) => void; reject: () => void }[] =
    [];
  let store = new TrailStore({
    getTrail: (since) =>
      new Promise((resolve, reject) => {
        requests.push({ since, resolve, reject: () => reject(new Error('read failed')) });
      }),
  });
  let subscriber = vi.fn<TrailSubscriber>();
  let unsubscribe = store.subscribe(subscriber);
  let ids = () => store.segments.featureCollection().features.map(({ id }) => Number(id) / MINUTE);
  let apply = (value: Trail | null) => store.apply({ topic: 'trail', value });
  let setLength = (trailLength: number) =>
    store.apply({ topic: 'settings', value: settingsFixture({ trailLength }) });
  return { store, requests, subscriber, unsubscribe, ids, apply, setLength };
}

afterEach(() => vi.restoreAllMocks());

describe('TrailStore', () => {
  it('sends each source change to its subscribers', async () => {
    let { store, requests, subscriber, unsubscribe, apply } = setup();
    apply(trail(1));
    requests[0].resolve([sample(0)]);
    await vi.waitFor(() => expect(subscriber).toHaveBeenCalled());

    store.apply({ topic: 'taskSaveFailed', value: false });
    apply(trail(2));
    unsubscribe();
    apply(null);

    expect(subscriber.mock.calls.map(([diff]) => diff.add?.map(({ id }) => id))).toEqual([
      [MINUTE],
      [2 * MINUTE],
    ]);
  });

  it('fetches the trail length before the first sample of a new subscription', async () => {
    let { requests, ids, apply } = setup();

    apply(trail(70));

    expect(requests.map(({ since }) => since)).toEqual([10 * MINUTE]);
    requests[0].resolve([10, 20, 30].map(sample));
    await vi.waitFor(() => expect(ids()).toEqual([20, 30, 70]));
  });

  it('merges the samples that arrive during the fetch by UTC without duplicates', async () => {
    let { requests, subscriber, ids, apply } = setup();
    apply(trail(2));
    apply(trail(3));

    requests[0].resolve([0, 1, 2, 3].map(sample));
    await vi.waitFor(() => expect(ids()).toEqual([1, 2, 3]));
    expect(subscriber).toHaveBeenCalledExactlyOnceWith({
      removeAll: true,
      add: [1, 2, 3].map((minutes) => expect.objectContaining({ id: minutes * MINUTE })),
    });

    apply(trail(3));
    apply(trail(4));
    expect(ids()).toEqual([1, 2, 3, 4]);
    expect(requests).toHaveLength(1);
  });

  it('keeps the maximum altitudes of the newest trail value', () => {
    let { store, apply } = setup();

    apply({ ...trail(1), maxAltitudeMslMeters: 1000, maxAltitudeAglMeters: 800 });
    apply({ ...trail(2), maxAltitudeMslMeters: 1200, maxAltitudeAglMeters: 900 });
    apply(null);

    expect(store.maxAltitudeMslMeters).toBe(1200);
    expect(store.maxAltitudeAglMeters).toBe(900);
  });

  it('fetches again when a new recording starts', async () => {
    let { requests, ids, apply } = setup();
    apply(trail(1));
    requests[0].resolve([0, 1].map(sample));
    await vi.waitFor(() => expect(ids()).toEqual([1]));

    apply(trail(5, 5 * MINUTE));

    expect(ids()).toEqual([]);
    expect(requests).toHaveLength(2);
    requests[1].resolve([sample(5)]);
    apply(trail(6, 5 * MINUTE));
    await vi.waitFor(() => expect(ids()).toEqual([6]));
  });

  it('fetches no sample from before the recording start', () => {
    let { requests, apply } = setup();

    apply(trail(65, 30 * MINUTE));

    expect(requests.map(({ since }) => since)).toEqual([30 * MINUTE]);
  });

  it('ignores the fetch result of a replaced recording', async () => {
    let { requests, ids, apply } = setup();
    apply(trail(1));
    apply(trail(5, 5 * MINUTE));
    apply(trail(6, 5 * MINUTE));
    requests[1].resolve([]);
    await vi.waitFor(() => expect(ids()).toEqual([6]));

    requests[0].resolve([0, 1].map(sample));
    await new Promise((resolve) => setTimeout(resolve));

    expect(ids()).toEqual([6]);
  });

  it('ignores the fetch result after the recording ends', async () => {
    let { store, requests, ids, apply } = setup();
    apply(trail(1));
    apply(null);

    requests[0].resolve([0, 1].map(sample));
    await new Promise((resolve) => setTimeout(resolve));

    expect(ids()).toEqual([]);
    expect(store.segments.recordingStart).toBeNull();
  });

  it('shows the buffered samples when the fetch fails', async () => {
    let warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    let { requests, ids, apply } = setup();
    apply(trail(1));
    apply(trail(2));

    requests[0].reject();

    await vi.waitFor(() => expect(ids()).toEqual([2]));
    expect(warn).toHaveBeenCalledExactlyOnceWith(
      'Trail resource request failed. Showing only the live samples.',
      new Error('read failed'),
    );
  });

  it('trims the trail locally for a shorter length', async () => {
    let { requests, subscriber, ids, apply, setLength } = setup();
    apply(trail(70));
    requests[0].resolve([10, 20, 30].map(sample));
    await vi.waitFor(() => expect(ids()).toEqual([20, 30, 70]));

    setLength(55);

    expect(ids()).toEqual([30, 70]);
    expect(subscriber).toHaveBeenLastCalledWith({ remove: [20 * MINUTE] });
    expect(requests).toHaveLength(1);
  });

  it('fetches the trail again for a longer length and continues it meanwhile', async () => {
    let { requests, ids, apply, setLength } = setup();
    apply(trail(70));
    requests[0].resolve([10, 20, 30].map(sample));
    await vi.waitFor(() => expect(ids()).toEqual([20, 30, 70]));

    setLength(65);
    apply(trail(71));

    expect(requests.map(({ since }) => since)).toEqual([10 * MINUTE, 5 * MINUTE]);
    expect(ids()).toEqual([20, 30, 70, 71]);
    requests[1].resolve([5, 8, 10, 20, 30, 70].map(sample));
    await vi.waitFor(() => expect(ids()).toEqual([10, 20, 30, 70, 71]));
  });

  it('keeps the trail when the fetch for a longer length fails', async () => {
    let warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    let { requests, ids, apply, setLength } = setup();
    apply(trail(70));
    requests[0].resolve([10, 20, 30].map(sample));
    await vi.waitFor(() => expect(ids()).toEqual([20, 30, 70]));
    setLength(65);
    apply(trail(71));

    requests[1].reject();
    await new Promise((resolve) => setTimeout(resolve));

    expect(ids()).toEqual([20, 30, 70, 71]);
    expect(warn).toHaveBeenCalledExactlyOnceWith(
      'Trail resource request failed. Keeping the current trail.',
      new Error('read failed'),
    );
    apply(trail(72));
    expect(ids()).toEqual([20, 30, 70, 71, 72]);
  });

  it('fetches again for a longer length during a pending fetch', async () => {
    let { requests, ids, apply, setLength } = setup();
    apply(trail(70));
    apply(trail(71));

    setLength(65);

    expect(requests.map(({ since }) => since)).toEqual([10 * MINUTE, 6 * MINUTE]);
    requests[0].resolve([10, 20].map(sample));
    requests[1].resolve([6, 10, 20].map(sample));
    await vi.waitFor(() => expect(ids()).toEqual([10, 20, 70, 71]));
  });

  it('shows no trail and fetches nothing for a length of 0', async () => {
    let { requests, subscriber, ids, apply, setLength } = setup();
    setLength(0);

    apply(trail(1));
    apply(trail(2));
    await new Promise((resolve) => setTimeout(resolve));

    expect(requests).toEqual([]);
    expect(ids()).toEqual([]);
    expect(subscriber).not.toHaveBeenCalled();
  });

  describe('drift', () => {
    let rebuild = { removeAll: true, add: [expect.objectContaining({ id: MINUTE })] };

    /** Returns a store that shows one segment, from 0 min to 1 min. */
    async function setupDrift() {
      let context = setup();
      let { store, requests, subscriber, apply } = context;
      apply(trail(1));
      requests[0].resolve([sample(0)]);
      await vi.waitFor(() => expect(subscriber).toHaveBeenCalled());
      subscriber.mockClear();

      let setDrift = (trailDriftCompensation: boolean) =>
        store.apply({ topic: 'settings', value: settingsFixture({ trailDriftCompensation }) });
      let setWind = (wind: DerivedWindInstruments | null) =>
        store.apply({
          topic: 'instruments',
          value: { ...EMPTY_INSTRUMENTS, derived: { ...EMPTY_DERIVED_INSTRUMENTS, wind } },
        });
      return { ...context, setDrift, setWind };
    }

    it('drifts the trail with the instruments wind only while drift compensation is on', async () => {
      let { store, subscriber, setDrift, setWind } = await setupDrift();

      setWind(WIND);
      expect(subscriber).not.toHaveBeenCalled();
      expect(store.translation).toBeNull();

      setDrift(true);
      expect(subscriber).toHaveBeenLastCalledWith(rebuild);
      expect(store.translation).toEqual(store.segments.translation());
      expect(store.translation).not.toBeNull();

      setDrift(false);
      expect(subscriber).toHaveBeenCalledTimes(2);
      expect(subscriber).toHaveBeenLastCalledWith(rebuild);
      expect(store.translation).toBeNull();
    });

    it('draws no drift without a wind estimate', async () => {
      let { store, subscriber, setDrift, setWind } = await setupDrift();

      setDrift(true);
      expect(subscriber).not.toHaveBeenCalled();
      expect(store.translation).toBeNull();

      setWind(WIND);
      setWind(null);
      expect(subscriber).toHaveBeenCalledTimes(2);
      expect(subscriber).toHaveBeenLastCalledWith(rebuild);
      expect(store.translation).toBeNull();
    });
  });
});
