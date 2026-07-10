import type { Change } from './generated/Change';
import type { Snapshot } from './generated/Snapshot';

export type StreamStatus = 'connecting' | 'live' | 'reconnecting';

export interface StateSubscriber {
  onSnapshot(snapshot: Snapshot): void;
  onChanges(changes: Change[]): void;
  onStatus(status: StreamStatus): void;
}

/**
 * Subscribes to the server's state stream (see docs/design/server.md).
 *
 * `EventSource` reconnects on its own, and every (re)connection starts
 * with a fresh snapshot, so no recovery logic is needed here — errors
 * only surface as a status change so the UI can show data staleness.
 * Returns an unsubscribe function.
 */
export function subscribeToState(subscriber: StateSubscriber): () => void {
  let source = new EventSource('/api/state');

  source.addEventListener('open', () => subscriber.onStatus('live'));
  source.addEventListener('error', () => subscriber.onStatus('reconnecting'));
  source.addEventListener('snapshot', (event: MessageEvent<string>) => {
    subscriber.onSnapshot(JSON.parse(event.data) as Snapshot);
  });
  source.addEventListener('changes', (event: MessageEvent<string>) => {
    subscriber.onChanges(JSON.parse(event.data) as Change[]);
  });

  return () => source.close();
}
