import type { GeoJSONSourceDiff } from 'maplibre-gl';
import type { UpdraftClient } from '#lib/client/index.js';
import type { Topic } from '#lib/protocol/generated/Topic.js';
import type { Trail } from '#lib/protocol/generated/Trail.js';
import type { TrailSample } from '#lib/protocol/generated/TrailSample.js';

import { TrailSegments, trailSince } from '#lib/map/trail.js';

export type TrailSubscriber = (diff: GeoJSONSourceDiff) => void;

/**
 * Fetches the trail resource for each recording that the store has not
 * shown yet, and continues it with the samples of the `trail` topic.
 */
export class TrailStore {
  #client: Pick<UpdraftClient, 'getTrail'>;
  #subscribers = new Set<TrailSubscriber>();
  /** The topic samples that arrive while the resource request is pending. */
  #pending: { recordingStart: number; samples: TrailSample[] } | null = null;

  segments = new TrailSegments();

  constructor(client: Pick<UpdraftClient, 'getTrail'>) {
    this.#client = client;
  }

  apply(topic: Topic): void {
    if (topic.topic !== 'trail') return;

    let trail = topic.value;
    if (trail && trail.recordingStart === this.#pending?.recordingStart) {
      this.#pending.samples.push(trail.sample);
    } else if (trail && trail.recordingStart !== this.segments.recordingStart) {
      void this.#fetch(trail);
    } else {
      this.#pending = null;
      this.#send(this.segments.apply(trail));
    }
  }

  subscribe(subscriber: TrailSubscriber): () => void {
    this.#subscribers.add(subscriber);

    return () => {
      this.#subscribers.delete(subscriber);
    };
  }

  async #fetch({ recordingStart, sample }: Trail): Promise<void> {
    let pending = { recordingStart, samples: [sample] };
    this.#pending = pending;
    this.#send(this.segments.apply(null));

    let samples: TrailSample[] = [];
    try {
      // A fetch that runs before the shell deletes the previous recording
      // reads its samples. They are older than the recording start.
      samples = await this.#client.getTrail(Math.max(trailSince(sample), recordingStart));
    } catch (error) {
      console.warn('Trail resource request failed. Showing only the live samples.', error);
    }

    if (this.#pending !== pending) return;
    this.#pending = null;
    this.#send(this.segments.replace(recordingStart, [...samples, ...pending.samples]));
  }

  #send(diff: GeoJSONSourceDiff | null): void {
    if (!diff) return;

    for (let subscriber of this.#subscribers) {
      subscriber(diff);
    }
  }
}
