import type { Settings } from '$lib/protocol/generated/Settings';
import type { Topic } from '$lib/protocol/generated/Topic';

const EMPTY: Settings = {
  locale: null,
  polar: 'LS 8',
  arrivalReserve: 200,
  units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
};

export class SettingsStore {
  current = $state.raw<Settings>(EMPTY);

  apply(topic: Topic): void {
    if (topic.topic !== 'settings') return;

    this.current = topic.value;
  }
}
