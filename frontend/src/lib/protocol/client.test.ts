import { describe, expect, it } from 'vitest';

import { HttpUpdraftClient } from './client';
import { ApplicationState } from './state.svelte';

class FakeEventSource {
  readonly listeners = new Map<string, Array<(event: Event) => void>>();
  closed = false;

  addEventListener(type: string, listener: (event: Event) => void): void {
    let listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  close(): void {
    this.closed = true;
  }

  emit(type: string, data?: string): void {
    let event = data === undefined ? new Event(type) : new MessageEvent(type, { data });
    for (let listener of this.listeners.get(type) ?? []) {
      listener(event);
    }
  }
}

describe('HttpUpdraftClient', () => {
  it('subscribes to the snapshot-first state stream', () => {
    let eventSource = new FakeEventSource();
    let requestedUrl: string | undefined;
    let client = new HttpUpdraftClient(
      (url) => {
        requestedUrl = url;
        return eventSource;
      },
      () => 2_000,
    );
    let state = new ApplicationState();

    let unsubscribe = client.subscribe(state);
    expect(requestedUrl).toBe('/api/state');

    eventSource.emit('open');
    expect(state.streamStatus).toBe('connected');
    expect(state.lastEventAtMs).toBeNull();

    eventSource.emit(
      'snapshot',
      JSON.stringify({
        flight: {
          position: null,
          pressureAltitudeMeters: null,
          traceStats: {
            fixCount: 2,
            distanceMeters: 42,
            maxAltitudeMeters: null,
          },
        },
      }),
    );

    expect(state.streamStatus).toBe('connected');
    expect(state.flight.traceStats?.fixCount).toBe(2);
    expect(state.lastEventAtMs).toBe(2_000);

    unsubscribe();
    expect(eventSource.closed).toBe(true);
  });

  it('forwards changes, heartbeats, and connection failures', () => {
    let eventSource = new FakeEventSource();
    let now = 3_000;
    let client = new HttpUpdraftClient(
      () => eventSource,
      () => now,
    );
    let state = new ApplicationState();
    client.subscribe(state);

    eventSource.emit(
      'changes',
      JSON.stringify([
        {
          group: 'flight',
          type: 'traceStats',
          value: { fixCount: 3, distanceMeters: 80, maxAltitudeMeters: 500 },
        },
      ]),
    );
    expect(state.flight.traceStats?.distanceMeters).toBe(80);
    expect(state.lastEventAtMs).toBe(3_000);

    now = 4_000;
    eventSource.emit('heartbeat', '{}');
    expect(state.lastEventAtMs).toBe(4_000);

    eventSource.emit('error');
    expect(state.streamStatus).toBe('reconnecting');
    expect(state.streamError?.message).toBe('State stream connection failed');
    expect(eventSource.closed).toBe(false);
  });

  it('closes the stream and reports malformed event data', () => {
    let eventSource = new FakeEventSource();
    let client = new HttpUpdraftClient(() => eventSource);
    let state = new ApplicationState();
    client.subscribe(state);

    eventSource.emit('snapshot', '{');

    expect(eventSource.closed).toBe(true);
    expect(state.streamStatus).toBe('failed');
    expect(state.streamError?.message).toBe('Invalid snapshot event from state stream');
  });
});
