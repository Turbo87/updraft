import type { Settings } from '#lib/protocol/generated/Settings.js';
import type { Topic } from '#lib/protocol/generated/Topic.js';

import { defaultSettings } from '#lib/settings.js';

export class SettingsStore {
  current = $state.raw<Settings>(defaultSettings());

  apply(topic: Topic): void {
    if (topic.topic !== 'settings') return;

    this.current = topic.value;
  }
}
