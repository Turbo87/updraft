import type { Locale } from '$lib/protocol/generated/Locale';
import type { Settings } from '$lib/protocol/generated/Settings';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
import type { TopicListener, UpdraftClient } from './index';

/** Drives the frontend without a Rust process behind it. */
export class FakeClient implements UpdraftClient {
  #listeners = new Set<TopicListener>();
  #settings: Settings = {
    locale: null,
    units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
  };

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

    this.#settings = { ...this.#settings, locale };
    this.emit({ topic: 'settings', value: this.#settings });
  }

  async setUnits(units: UnitSettings): Promise<void> {
    let current = this.#settings.units;
    if (
      current.altitude === units.altitude &&
      current.distance === units.distance &&
      current.speed === units.speed &&
      current.verticalSpeed === units.verticalSpeed
    ) {
      return;
    }

    this.#settings = { ...this.#settings, units: { ...units } };
    this.emit({ topic: 'settings', value: this.#settings });
  }

  /** Publishes a topic as though the core had emitted it. */
  emit(topic: Topic): void {
    for (let listener of this.#listeners) {
      listener(topic);
    }
  }
}
