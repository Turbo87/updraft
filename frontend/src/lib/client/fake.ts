import type { Locale } from '$lib/protocol/generated/Locale';
import type { Settings } from '$lib/protocol/generated/Settings';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { TopicListener, UpdraftClient } from './index';

/** Drives the frontend without a Rust process behind it. */
export class FakeClient implements UpdraftClient {
  #listeners = new Set<TopicListener>();
  #settings: Settings = { locale: null };

  subscribe(onTopic: TopicListener): () => void {
    this.#listeners.add(onTopic);
    onTopic({ topic: 'settings', value: this.#settings });
    onTopic({ topic: 'traffic', value: { type: 'snapshot', value: [] } });

    return () => {
      this.#listeners.delete(onTopic);
    };
  }

  async setLocale(locale: Locale): Promise<void> {
    if (this.#settings.locale === locale) return;

    this.#settings = { locale };
    this.emit({ topic: 'settings', value: this.#settings });
  }

  /** Publishes a topic as though the core had emitted it. */
  emit(topic: Topic): void {
    for (let listener of this.#listeners) {
      listener(topic);
    }
  }
}
