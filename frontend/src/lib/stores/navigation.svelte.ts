import type { Navigation } from '$lib/protocol/generated/Navigation';
import type { Topic } from '$lib/protocol/generated/Topic';

export class NavigationStore {
  current = $state.raw<Navigation | null>(null);
  apply(topic: Topic): void {
    if (topic.topic === 'navigation') this.current = topic.value;
  }
}
