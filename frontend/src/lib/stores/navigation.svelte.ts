import type { Navigation } from '#lib/protocol/generated/Navigation.js';
import type { NavigationTarget } from '#lib/protocol/generated/NavigationTarget.js';
import type { PinnedTarget } from '#lib/protocol/generated/PinnedTarget.js';
import type { PublishedTask } from '#lib/protocol/generated/PublishedTask.js';
import type { Topic } from '#lib/protocol/generated/Topic.js';

export class NavigationStore {
  task = $state.raw<PublishedTask>({
    points: [],
    nextId: 0,
    target: null,
    progress: { reached: 0, start: null, finish: null },
  });
  taskSaveFailed = $state(false);
  recents = $state.raw<NavigationTarget[]>([]);
  pins = $state.raw<PinnedTarget[]>([]);
  current = $state.raw<Navigation | null>(null);
  apply(topic: Topic): void {
    if (topic.topic === 'taskSaveFailed') this.taskSaveFailed = topic.value;
    if (topic.topic === 'task') this.task = topic.value;
    if (topic.topic === 'recentTargets') this.recents = topic.value;
    if (topic.topic === 'pinnedTargets') this.pins = topic.value;
    if (topic.topic === 'navigation') this.current = topic.value;
  }
}
