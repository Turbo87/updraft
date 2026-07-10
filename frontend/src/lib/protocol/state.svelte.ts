import { subscribeToState, type StreamStatus } from './client';
import type { Change } from './generated/Change';
import type { OwnshipPosition } from './generated/OwnshipPosition';
import type { Snapshot } from './generated/Snapshot';

/**
 * The client-side mirror of the core's state stream: seeded by the
 * snapshot, updated by changes, exposed as reactive state.
 */
export class ApplicationState {
  position = $state.raw<OwnshipPosition | null>(null);
  streamStatus = $state<StreamStatus>('connecting');
  /** Timestamp (ms) of the last stream message, for data-age display. */
  lastUpdatedAt = $state<number | null>(null);

  applySnapshot(snapshot: Snapshot) {
    this.position = snapshot.position;
    this.lastUpdatedAt = Date.now();
  }

  applyChanges(changes: Change[]) {
    for (let change of changes) {
      this.position = change.flight.position_changed;
    }
    this.lastUpdatedAt = Date.now();
  }
}

/** Connects `state` to the server's state stream; returns a disconnect function. */
export function connectApplicationState(state: ApplicationState): () => void {
  return subscribeToState({
    onSnapshot: (snapshot) => state.applySnapshot(snapshot),
    onChanges: (changes) => state.applyChanges(changes),
    onStatus: (status) => (state.streamStatus = status),
  });
}
