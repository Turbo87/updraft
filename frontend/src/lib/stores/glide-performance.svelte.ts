import type { GlidePerformance } from '$lib/protocol/generated/GlidePerformance';
import type { Topic } from '$lib/protocol/generated/Topic';

export class GlidePerformanceStore {
  current = $state.raw<GlidePerformance>({ macCready: 0, bugs: 0, ballast: 0 });

  apply(topic: Topic): void {
    if (topic.topic === 'glidePerformance') this.current = topic.value;
  }
}
