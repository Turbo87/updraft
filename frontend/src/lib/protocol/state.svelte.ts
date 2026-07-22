import type { StateSubscription } from './client';
import type { Change } from './generated/Change';
import type { FlightChange } from './generated/FlightChange';
import type { GnssState } from './generated/GnssState';
import type { Availability } from './generated/Availability';
import type { Snapshot } from './generated/Snapshot';
import type { TraceStats } from './generated/TraceStats';

export type StreamStatus = 'connecting' | 'connected' | 'reconnecting' | 'failed';

export class FlightState {
  gnss = $state.raw<Availability<GnssState>>({ status: 'unavailable' });
  pressureAltitudeMeters = $state.raw<Availability<number>>({ status: 'unavailable' });
  traceStats = $state.raw<TraceStats | null>(null);

  replace(snapshot: Snapshot['flight']): void {
    this.gnss = snapshot.gnss;
    this.pressureAltitudeMeters = snapshot.pressureAltitudeMeters;
    this.traceStats = snapshot.traceStats;
  }

  apply(change: FlightChange): void {
    switch (change.type) {
      case 'gnss':
        this.gnss = change.value;
        break;
      case 'pressureAltitudeMeters':
        this.pressureAltitudeMeters = change.value;
        break;
      case 'traceStats':
        this.traceStats = change.value;
        break;
      default:
        unexpectedChange(change);
    }
  }
}

/** Reactive frontend state seeded and updated by the server state stream. */
export class ApplicationState implements StateSubscription {
  readonly flight = new FlightState();
  streamStatus = $state<StreamStatus>('connecting');
  streamError = $state.raw<Error | null>(null);
  lastEventAtMs = $state<number | null>(null);

  applySnapshot(snapshot: Snapshot, receivedAtMs: number): void {
    this.flight.replace(snapshot.flight);
    this.markActivity(receivedAtMs);
  }

  streamOpened(): void {
    this.streamStatus = 'connected';
    this.streamError = null;
  }

  applyChanges(changes: Change[], receivedAtMs: number): void {
    // `Change` currently contains only the flight group. A new generated group
    // will make this call fail to type-check until its store is added here.
    for (let change of changes) {
      this.flight.apply(change);
    }

    this.markActivity(receivedAtMs);
  }

  recordHeartbeat(receivedAtMs: number): void {
    this.markActivity(receivedAtMs);
  }

  connectionFailed(error: Error): void {
    this.streamStatus = 'reconnecting';
    this.streamError = error;
  }

  protocolFailed(error: Error): void {
    this.streamStatus = 'failed';
    this.streamError = error;
  }

  dataAgeMs(nowMs: number): number | null {
    if (this.lastEventAtMs === null) {
      return null;
    }

    return Math.max(0, nowMs - this.lastEventAtMs);
  }

  private markActivity(receivedAtMs: number): void {
    this.lastEventAtMs = receivedAtMs;
    this.streamStatus = 'connected';
    this.streamError = null;
  }
}

function unexpectedChange(change: never): never {
  throw new Error(`Unsupported state change: ${JSON.stringify(change)}`);
}
