import type { PublishedTrafficTarget } from '$lib/protocol/generated/PublishedTrafficTarget';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { TrafficUpdate } from '$lib/protocol/generated/TrafficUpdate';

import { SvelteMap } from 'svelte/reactivity';

export type TrafficSubscriber = (
  update: TrafficUpdate,
  currentTargets: ReadonlyMap<string, PublishedTrafficTarget>,
) => void;

export class TrafficStore {
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- Subscriptions do not affect rendering.
  #subscribers = new Set<TrafficSubscriber>();

  current = new SvelteMap<string, PublishedTrafficTarget>();

  apply(topic: Topic): void {
    if (topic.topic !== 'traffic') return;

    if (topic.value.type === 'snapshot') {
      this.current.clear();
      for (let target of topic.value.value) {
        this.current.set(target.id, target);
      }
    } else {
      for (let target of topic.value.value.upserts) {
        this.current.set(target.id, target);
      }

      for (let id of topic.value.value.removed) {
        this.current.delete(id);
      }
    }

    for (let subscriber of this.#subscribers) {
      subscriber(topic.value, this.current);
    }
  }

  subscribe(subscriber: TrafficSubscriber): () => void {
    this.#subscribers.add(subscriber);

    return () => {
      this.#subscribers.delete(subscriber);
    };
  }
}
