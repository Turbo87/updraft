import { describe, expect, it } from 'vitest';
import { ApplicationState } from './state.svelte';

describe('ApplicationState', () => {
  it('replaces flight state from a snapshot', () => {
    let state = new ApplicationState();

    state.applySnapshot(
      {
        flight: {
          position: {
            observedAtMs: 1_250,
            latitudeDegrees: 50.823,
            longitudeDegrees: 6.186,
            altitudeMeters: 400.5,
            trackDegrees: 45,
            groundSpeedMetersPerSecond: 30,
          },
          traceStats: null,
        },
      },
      2_000,
    );

    expect(state.flight.position).toEqual({
      observedAtMs: 1_250,
      latitudeDegrees: 50.823,
      longitudeDegrees: 6.186,
      altitudeMeters: 400.5,
      trackDegrees: 45,
      groundSpeedMetersPerSecond: 30,
    });
    expect(state.flight.traceStats).toBeNull();
    expect(state.streamStatus).toBe('connected');
    expect(state.lastEventAtMs).toBe(2_000);
    expect(state.dataAgeMs(2_750)).toBe(750);
  });

  it('applies every change in a batch', () => {
    let state = new ApplicationState();

    state.applyChanges(
      [
        {
          group: 'flight',
          type: 'position',
          value: {
            observedAtMs: 3_000,
            latitudeDegrees: 50.824,
            longitudeDegrees: 6.187,
            altitudeMeters: null,
            trackDegrees: null,
            groundSpeedMetersPerSecond: null,
          },
        },
        {
          group: 'flight',
          type: 'traceStats',
          value: {
            fixCount: 4,
            distanceMeters: 123.5,
            maxAltitudeMeters: 450,
          },
        },
      ],
      3_100,
    );

    expect(state.flight.position?.latitudeDegrees).toBe(50.824);
    expect(state.flight.traceStats).toEqual({
      fixCount: 4,
      distanceMeters: 123.5,
      maxAltitudeMeters: 450,
    });
    expect(state.lastEventAtMs).toBe(3_100);
  });

  it('clears trace statistics when the core invalidates them', () => {
    let state = new ApplicationState();
    state.applySnapshot(
      {
        flight: {
          position: null,
          traceStats: {
            fixCount: 4,
            distanceMeters: 123.5,
            maxAltitudeMeters: 450,
          },
        },
      },
      3_100,
    );

    state.applyChanges([{ group: 'flight', type: 'traceStats', value: null }], 3_200);

    expect(state.flight.traceStats).toBeNull();
  });

  it('surfaces stream failures until activity resumes', () => {
    let state = new ApplicationState();
    let error = new Error('state stream connection failed');

    state.connectionFailed(error);

    expect(state.streamStatus).toBe('reconnecting');
    expect(state.streamError).toBe(error);
    expect(state.dataAgeMs(4_000)).toBeNull();

    state.recordHeartbeat(4_000);

    expect(state.streamStatus).toBe('connected');
    expect(state.streamError).toBeNull();
    expect(state.lastEventAtMs).toBe(4_000);
  });
});
