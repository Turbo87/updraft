import type { Navigation } from '$lib/protocol/generated/Navigation';
import type { NavigationTarget } from '$lib/protocol/generated/NavigationTarget';
import type { PinnedTarget } from '$lib/protocol/generated/PinnedTarget';
import type { Task } from '$lib/protocol/generated/Task';
import type { Topic } from '$lib/protocol/generated/Topic';

export class NavigationStore {
  task = $state.raw<Task>({
    points: [],
    current: null,
    status: 'stopped',
    nextId: 0,
    start: null,
    finish: null,
    restartAllowed: false,
  });
  recents = $state.raw<NavigationTarget[]>([]);
  pins = $state.raw<PinnedTarget[]>([]);
  current = $state.raw<Navigation | null>(null);
  apply(topic: Topic): void {
    if (topic.topic === 'task') this.task = topic.value;
    if (topic.topic === 'recentTargets') this.recents = topic.value;
    if (topic.topic === 'pinnedTargets') this.pins = topic.value;
    if (topic.topic === 'navigation') this.current = topic.value;
  }
}
