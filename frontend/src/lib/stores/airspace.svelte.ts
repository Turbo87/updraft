import type { AirspaceStatus } from '$lib/protocol/generated/AirspaceStatus';
import type { Topic } from '$lib/protocol/generated/Topic';

export class AirspaceStore {
  current = $state.raw<AirspaceStatus>({ type: 'none' });

  apply(topic: Topic): void {
    if (topic.topic !== 'airspace') return;

    this.current = topic.value;
  }
}
