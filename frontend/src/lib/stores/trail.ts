import type { GeoJSONSourceDiff } from 'maplibre-gl';
import type { Topic } from '#lib/protocol/generated/Topic.js';

import { TrailSegments } from '#lib/map/trail.js';

export type TrailSubscriber = (diff: GeoJSONSourceDiff) => void;

export class TrailStore {
  #subscribers = new Set<TrailSubscriber>();

  segments = new TrailSegments();

  apply(topic: Topic): void {
    if (topic.topic !== 'trail') return;

    let diff = this.segments.apply(topic.value);
    if (!diff) return;

    for (let subscriber of this.#subscribers) {
      subscriber(diff);
    }
  }

  subscribe(subscriber: TrailSubscriber): () => void {
    this.#subscribers.add(subscriber);

    return () => {
      this.#subscribers.delete(subscriber);
    };
  }
}
