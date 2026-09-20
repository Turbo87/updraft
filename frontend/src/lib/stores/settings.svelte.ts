import type { Settings } from '$lib/protocol/generated/Settings';
import type { Topic } from '$lib/protocol/generated/Topic';

import { defaultSettings } from '$lib/settings';

export class SettingsStore {
  current = $state.raw<Settings>(defaultSettings());

  apply(topic: Topic): void {
    if (topic.topic !== 'settings') return;

    this.current = topic.value;
  }
}
