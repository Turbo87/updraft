import { subscribeToState, type StreamStatus } from './client';
import type { Change } from './generated/Change';
import type { OwnshipPosition } from './generated/OwnshipPosition';
import type { PositionFix } from './generated/PositionFix';
import type { Snapshot } from './generated/Snapshot';

/**
 * The client-side mirror of the core's state stream: seeded by the
 * snapshot, updated by changes, exposed as reactive state.
 */
export class ApplicationState {
  fix = $state.raw<PositionFix | null>(null);
  /** Flown-track distance in meters. */
  trackDistance = $state(0);
  streamStatus = $state<StreamStatus>('connecting');
  /** Timestamp (ms) of the last stream message, for data-age display. */
  lastUpdatedAt = $state<number | null>(null);

  position = $derived.by((): OwnshipPosition | null => {
    if (this.fix === null) return null;
    return 'current' in this.fix ? this.fix.current : this.fix.stale;
  });

  positionStale = $derived(this.fix !== null && 'stale' in this.fix);

  applySnapshot(snapshot: Snapshot) {
    this.fix = snapshot.position;
    this.trackDistance = snapshot.track_distance;
    this.lastUpdatedAt = Date.now();
  }

  applyChanges(changes: Change[]) {
    for (let change of changes) {
      let flight = change.flight;
      if (flight === 'position_stale') {
        if (this.position !== null) {
          this.fix = { stale: this.position };
        }
      } else if ('position_changed' in flight) {
        this.fix = { current: flight.position_changed };
      } else {
        this.trackDistance = flight.track_distance_changed;
      }
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
