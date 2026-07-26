import type { Instruments } from '$lib/protocol/generated/Instruments';
import type { Topic } from '$lib/protocol/generated/Topic';

const EMPTY: Instruments = {
  position: null,
  trackDegrees: null,
  groundSpeedMetersPerSecond: null,
  altitudeMslMeters: null,
};

/**
 * Holds the latest instruments topic.
 *
 * Topics arrive whole, so the store replaces rather than merges and the
 * view is a pure function of the last message received.
 */
export class InstrumentsStore {
  current = $state.raw<Instruments>(EMPTY);

  apply(topic: Topic): void {
    if (topic.topic !== 'instruments') return;

    this.current = topic.value;
  }
}
