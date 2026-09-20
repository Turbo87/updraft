import type { Navigation } from '$lib/protocol/generated/Navigation';
import type { PinnedTarget } from '$lib/protocol/generated/PinnedTarget';
import type { Topic } from '$lib/protocol/generated/Topic';

export class NavigationStore {
  pins = $state.raw<PinnedTarget[]>([]);
  current = $state.raw<Navigation | null>(null);
  apply(topic: Topic): void {
    if (topic.topic === 'pinnedTargets') this.pins = topic.value;
    if (topic.topic === 'navigation') this.current = topic.value;
  }
}
