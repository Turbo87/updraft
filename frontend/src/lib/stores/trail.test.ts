import type { Trail } from '#lib/protocol/generated/Trail.js';
import type { TrailSample } from '#lib/protocol/generated/TrailSample.js';
import type { TrailSubscriber } from './trail';

import { afterEach, describe, expect, it, vi } from 'vitest';

import { TrailStore } from './trail';

const MINUTE = 60_000;

function sample(minutes: number): TrailSample {
  return {
    unixMilliseconds: minutes * MINUTE,
    position: { latitudeDegrees: 50 + minutes / 1000, longitudeDegrees: 6 },
    altitudeMslMeters: null,
    altitudeAglMeters: null,
    varioMetersPerSecond: null,
    nettoMetersPerSecond: null,
    relativeVarioMetersPerSecond: null,
  };
}

function trail(minutes: number, recordingStart = 0): Trail {
  return { recordingStart, sample: sample(minutes) };
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
  return { store, requests, subscriber, unsubscribe, ids, apply };
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
});
