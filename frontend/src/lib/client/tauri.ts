import type { Locale } from '$lib/protocol/generated/Locale';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';
import type { TopicListener, UpdraftClient } from './index';

import { Channel, invoke } from '@tauri-apps/api/core';

export class TauriClient implements UpdraftClient {
  setLocale(locale: Locale): Promise<void> {
    return invoke('set_locale', { locale });
  }

  setUnits(units: UnitSettings): Promise<void> {
    return invoke('set_units', { units });
  }

  subscribe(onTopic: TopicListener): () => void {
    let channel = new Channel<Topic>();
    channel.onmessage = onTopic;

    let closed = false;
    void invoke('subscribe', { channel }).catch((error: unknown) => {
      if (!closed) {
        console.error('Failed to subscribe to the state stream', error);
      }
    });

    return () => {
      closed = true;
      channel.onmessage = () => {};
    };
  }
}
