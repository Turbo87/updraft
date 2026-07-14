import type { Change } from './generated/Change';
import type { Snapshot } from './generated/Snapshot';

type StateEventName = 'snapshot' | 'changes';
type ParsedEvent<T> = { ok: true; value: T } | { ok: false; error: Error };

export interface EventSourceLike {
  addEventListener(type: string, listener: (event: Event) => void): void;
  close(): void;
}

export type EventSourceFactory = (url: string) => EventSourceLike;

export interface StateSubscription {
  streamOpened(): void;
  applySnapshot(snapshot: Snapshot, receivedAtMs: number): void;
  applyChanges(changes: Change[], receivedAtMs: number): void;
  recordHeartbeat(receivedAtMs: number): void;
  connectionFailed(error: Error): void;
  protocolFailed(error: Error): void;
}

export interface UpdraftClient {
  subscribe(subscription: StateSubscription): () => void;
}

export class HttpUpdraftClient implements UpdraftClient {
  constructor(
    private readonly createEventSource: EventSourceFactory = openEventSource,
    private readonly now: () => number = Date.now,
  ) {}

  subscribe(subscription: StateSubscription): () => void {
    let eventSource = this.createEventSource('/api/state');

    eventSource.addEventListener('open', () => subscription.streamOpened());
    eventSource.addEventListener('snapshot', (event) => {
      let result = parseEvent<Snapshot>(event, 'snapshot');
      if (result.ok) {
        subscription.applySnapshot(result.value, this.now());
      } else {
        eventSource.close();
        subscription.protocolFailed(result.error);
      }
    });
    eventSource.addEventListener('changes', (event) => {
      let result = parseEvent<Change[]>(event, 'changes');
      if (result.ok) {
        subscription.applyChanges(result.value, this.now());
      } else {
        eventSource.close();
        subscription.protocolFailed(result.error);
      }
    });
    eventSource.addEventListener('heartbeat', () => subscription.recordHeartbeat(this.now()));
    eventSource.addEventListener('error', () => {
      subscription.connectionFailed(new Error('State stream connection failed'));
    });

    return function unsubscribe() {
      eventSource.close();
    };
  }
}

function openEventSource(url: string): EventSourceLike {
  return new EventSource(url);
}

function parseEvent<T>(event: Event, eventName: StateEventName): ParsedEvent<T> {
  try {
    let value = JSON.parse((event as MessageEvent<string>).data) as T;
    return { ok: true, value };
  } catch (cause) {
    return {
      ok: false,
      error: new Error(`Invalid ${eventName} event from state stream`, { cause }),
    };
  }
}
