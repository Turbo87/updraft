import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';
import type { Topic } from '$lib/protocol/generated/Topic';

export class AirspaceStore {
  current = $state.raw<AirspaceStatus>({ generation: 0, sources: [] });
  initialized = $state(false);

  apply(topic: Topic): void {
    if (topic.topic !== 'airspace') return;

    this.current = topic.value;
    this.initialized = true;
  }
}
