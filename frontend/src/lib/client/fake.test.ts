import type { Topic } from '$lib/protocol/generated/Topic';

import { describe, expect, it, vi } from 'vitest';

import { FakeClient } from './fake';

function instruments(trackDegrees: number): Topic {
  return {
    topic: 'instruments',
    value: {
      position: { latitudeDegrees: 50.823, longitudeDegrees: 6.186 },
      trackDegrees,
      groundSpeedMetersPerSecond: null,
      altitudeMslMeters: null,
    },
  };
}

describe('FakeClient', () => {
  it('delivers emitted topics to a subscriber', () => {
    let client = new FakeClient();
    let received: Topic[] = [];

    client.subscribe((topic) => received.push(topic));
    received = [];
    client.emit(instruments(270));

    expect(received).toEqual([instruments(270)]);
  });

  it('stops delivering after unsubscribe', () => {
    let client = new FakeClient();
    let onTopic = vi.fn();

    let unsubscribe = client.subscribe(onTopic);
    onTopic.mockClear();
    unsubscribe();
    client.emit(instruments(90));

    expect(onTopic).not.toHaveBeenCalled();
  });

  it('delivers onboarding topics when a subscriber connects', () => {
    let client = new FakeClient();
    let received: Topic[] = [];

    client.subscribe((topic) => received.push(topic));

    expect(received).toEqual([
      {
        topic: 'settings',
        value: {
          locale: null,
          units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
        },
      },
      {
        topic: 'traffic',
        value: { type: 'snapshot', value: [] },
      },
    ]);
  });

  it('publishes an explicit locale through the settings topic', async () => {
    let client = new FakeClient();
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));

    await client.setLocale('de');

    expect(received.at(-1)).toEqual({
      topic: 'settings',
      value: {
        locale: 'de',
        units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
      },
    });
  });

  it('does not republish the active explicit locale', async () => {
    let client = new FakeClient();
    let received: Topic[] = [];
    client.subscribe((topic) => received.push(topic));
    received = [];

    await client.setLocale('de');
    await client.setLocale('de');

    expect(received).toHaveLength(1);
  });
});
