import type { Topic } from '$lib/protocol/generated/Topic';
import type { TopicListener, UpdraftClient } from './index';

/** Drives the frontend without a Rust process behind it. */
export class FakeClient implements UpdraftClient {
  #listeners = new Set<TopicListener>();

  subscribe(onTopic: TopicListener): () => void {
    this.#listeners.add(onTopic);

    return () => {
      this.#listeners.delete(onTopic);
    };
  }

  /** Publishes a topic as though the core had emitted it. */
  emit(topic: Topic): void {
    for (let listener of this.#listeners) {
      listener(topic);
    }
  }
}
