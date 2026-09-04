import type { Topic } from '$lib/protocol/generated/Topic';
import type { WaypointStatus } from '$lib/protocol/generated/WaypointStatus';

export class WaypointsStore {
  current = $state.raw<WaypointStatus>({ generation: 0, sources: [] });
  initialized = $state(false);

  apply(topic: Topic): void {
    if (topic.topic !== 'waypoints') return;
    this.current = topic.value;
    this.initialized = true;
  }
}
