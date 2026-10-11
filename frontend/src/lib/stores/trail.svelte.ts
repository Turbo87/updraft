import type { GeoJSONSourceDiff } from 'maplibre-gl';
import type { UpdraftClient } from '#lib/client/index.js';
import type { TrailTranslation } from '#lib/map/trail.js';
import type { DerivedWindInstruments } from '#lib/protocol/generated/DerivedWindInstruments.js';
import type { Topic } from '#lib/protocol/generated/Topic.js';
import type { TrailLength } from '#lib/protocol/generated/TrailLength.js';
import type { TrailSample } from '#lib/protocol/generated/TrailSample.js';

import { TrailSegments } from '#lib/map/trail.js';

export type TrailSubscriber = (diff: GeoJSONSourceDiff) => void;

/**
 * Fetches the trail resource for each recording that the store has not
 * shown yet, and continues it with the samples of the `trail` topic.
 */
export class TrailStore {
  #client: Pick<UpdraftClient, 'getTrail'>;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- Subscriptions do not affect rendering.
  #subscribers = new Set<TrailSubscriber>();
  /** The topic samples that arrive while the resource request is pending. */
  #pending: { recordingStart: number; samples: TrailSample[] } | null = null;
  #driftCompensation = false;
  #wind: DerivedWindInstruments | null = null;

  segments = new TrailSegments();
  hasRecording = $state(false);
  maxAltitudeMslMeters = $state<number | null>(null);
  maxAltitudeAglMeters = $state<number | null>(null);
  translation = $state.raw<TrailTranslation | null>(null);

  constructor(client: Pick<UpdraftClient, 'getTrail'>) {
    this.#client = client;
  }

  apply(topic: Topic): void {
    if (topic.topic === 'settings') {
      this.#resize(topic.value.trailLength);
      this.#driftCompensation = topic.value.trailDriftCompensation;
      this.#drift();
    }
    if (topic.topic === 'instruments') {
      this.#wind = topic.value.derived?.wind ?? null;
      this.#drift();
    }
    if (topic.topic !== 'trail') return;

    let trail = topic.value;
    this.hasRecording = trail !== null;
    if (trail) {
      this.maxAltitudeMslMeters = trail.maxAltitudeMslMeters;
      this.maxAltitudeAglMeters = trail.maxAltitudeAglMeters;
    }

    if (trail && trail.recordingStart === this.#pending?.recordingStart) {
      this.#pending.samples.push(trail.sample);
      // During the fetch for a longer length, the current trail stays visible.
      if (trail.recordingStart === this.segments.recordingStart) {
        this.#send(this.segments.apply(trail));
      }
    } else if (trail && trail.recordingStart !== this.segments.recordingStart) {
      this.#send(this.segments.apply(null));
      void this.#fetch(trail.recordingStart, [trail.sample]);
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

  /**
   * Trims the trail locally for a shorter length. A longer length fetches
   * the trail again. The current trail stays visible until the response.
   */
  #resize(length: TrailLength): void {
    let longer = length > this.segments.length;
    this.#send(this.segments.resize(length));
    if (!longer) return;

    let { recordingStart, newest } = this.segments;
    if (this.#pending) {
      void this.#fetch(this.#pending.recordingStart, this.#pending.samples);
    } else if (recordingStart !== null && newest) {
      void this.#fetch(recordingStart, [newest]);
    }
  }

  #drift(): void {
    this.#send(this.segments.drift(this.#driftCompensation ? this.#wind : null));
  }

  /** `samples` are the topic samples in arrival order, so the last is the newest. */
  async #fetch(recordingStart: number, samples: TrailSample[]): Promise<void> {
    let pending = { recordingStart, samples };
    this.#pending = pending;

    let fetched: TrailSample[] = [];
    let keep = false;
    if (this.segments.length > 0) {
      let newest = samples[samples.length - 1];
      try {
        // A fetch that runs before the shell deletes the previous recording
        // reads its samples. They are older than the recording start.
        fetched = await this.#client.getTrail(
          Math.max(this.segments.since(newest), recordingStart),
        );
      } catch (error) {
        keep = this.segments.recordingStart === recordingStart;
        let fallback = keep ? 'Keeping the current trail.' : 'Showing only the live samples.';
        console.warn(`Trail resource request failed. ${fallback}`, error);
      }
    }

    if (this.#pending !== pending) return;
    this.#pending = null;
    if (keep) return;
    this.#send(this.segments.replace(recordingStart, [...fetched, ...pending.samples]));
  }

  #send(diff: GeoJSONSourceDiff | null): void {
    let translation = this.segments.translation();
    if (!sameTranslation(translation, this.translation)) this.translation = translation;
    if (!diff) return;

    for (let subscriber of this.#subscribers) {
      subscriber(diff);
    }
  }
}

function sameTranslation(a: TrailTranslation | null, b: TrailTranslation | null): boolean {
  return (
    a?.eastMeters === b?.eastMeters &&
    a?.northMeters === b?.northMeters &&
    a?.latitudeDegrees === b?.latitudeDegrees
  );
}
