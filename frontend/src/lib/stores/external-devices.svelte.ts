import type { PublishedExternalDevice } from '$lib/protocol/generated/PublishedExternalDevice';
import type { Topic } from '$lib/protocol/generated/Topic';

/** Holds the complete ordered external-device list from the latest topic. */
export class ExternalDevicesStore {
  current = $state.raw<PublishedExternalDevice[]>([]);
  initialized = $state(false);

  apply(topic: Topic): void {
    if (topic.topic !== 'externalDevices') return;

    this.current = topic.value;
    this.initialized = true;
  }
}
