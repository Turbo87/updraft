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
    client.emit(instruments(270));

    expect(received).toEqual([instruments(270)]);
  });

  it('stops delivering after unsubscribe', () => {
    let client = new FakeClient();
    let onTopic = vi.fn();

    let unsubscribe = client.subscribe(onTopic);
    unsubscribe();
    client.emit(instruments(90));

    expect(onTopic).not.toHaveBeenCalled();
  });
});
