import type { UpdraftClient } from '$lib/client';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { AirspaceStore } from './airspace.svelte';
import type { WaypointsStore } from './waypoints.svelte';

import { SvelteMap, SvelteSet } from 'svelte/reactivity';

type DatasetType = 'airspace' | 'waypoints';
type Change = { type: DatasetType; name: string; enabled: boolean };
type ActivationClient = Pick<UpdraftClient, 'setAirspaceEnabled' | 'setWaypointsEnabled'>;

export class DataActivation {
  pending = $state(false);
  #requests = new SvelteMap<string, Change>();
  #errors = new SvelteSet<string>();
  #queue: Change[] = [];
  #waiting?: { change: Change; generation: number; resolve: () => void };

  constructor(
    private client: ActivationClient,
    private airspace: AirspaceStore,
    private waypoints: WaypointsStore,
  ) {}

  isEnabled(type: DatasetType, source: { sourceName: string; type: string }): boolean {
    return (
      this.#requests.get(this.#key(type, source.sourceName))?.enabled ?? source.type !== 'disabled'
    );
  }

  hasError(type: DatasetType, name: string): boolean {
    return this.#errors.has(this.#key(type, name));
  }

  setEnabled(type: DatasetType, name: string, enabled: boolean): void {
    let change = { type, name, enabled };
    let key = this.#key(type, name);
    this.#requests.set(key, change);
    this.#errors.delete(key);
    this.#queue.push(change);
    void this.#drain();
  }

  apply(topic: Topic): void {
    if (topic.topic !== 'airspace' && topic.topic !== 'waypoints') return;
    let keys = topic.value.sources.map((source) => this.#key(topic.topic, source.sourceName));
    for (let key of this.#errors) {
      if (key.startsWith(`${topic.topic}:`) && !keys.includes(key)) this.#errors.delete(key);
    }
    if (topic.topic === this.#waiting?.change.type) this.#resolvePublished();
  }

  #key(type: DatasetType, name: string): string {
    return `${type}:${name}`;
  }

  #status(type: DatasetType) {
    return type === 'airspace' ? this.airspace.current : this.waypoints.current;
  }

  #resolvePublished(): void {
    let waiting = this.#waiting;
    if (!waiting) return;
    let status = this.#status(waiting.change.type);
    let source = status.sources.find((source) => source.sourceName === waiting.change.name);
    if (
      status.generation <= waiting.generation ||
      (source && (source.type !== 'disabled') !== waiting.change.enabled)
    )
      return;
    this.#waiting = undefined;
    waiting.resolve();
  }

  async #drain(): Promise<void> {
    if (this.pending) return;
    this.pending = true;
    while (this.#queue.length) {
      let change = this.#queue.shift()!;
      let key = this.#key(change.type, change.name);
      let status = this.#status(change.type);
      if (status.sources.some((source) => source.sourceName === change.name)) {
        try {
          // The topic and command reply can arrive in either order.
          let published = new Promise<void>((resolve) => {
            this.#waiting = { change, generation: status.generation, resolve };
          });
          if (change.type === 'airspace')
            await this.client.setAirspaceEnabled(change.name, change.enabled);
          else await this.client.setWaypointsEnabled(change.name, change.enabled);
          await published;
        } catch {
          this.#waiting = undefined;
          let exists = this.#status(change.type).sources.some(
            (source) => source.sourceName === change.name,
          );
          if (exists && this.#requests.get(key) === change) this.#errors.add(key);
        }
      }
      if (this.#requests.get(key) === change) this.#requests.delete(key);
    }
    this.pending = false;
  }
}
