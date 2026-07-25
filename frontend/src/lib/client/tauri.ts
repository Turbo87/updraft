import type { Topic } from '$lib/protocol/generated/Topic';
import type { TopicListener, UpdraftClient } from './index';

import { Channel, invoke } from '@tauri-apps/api/core';

export class TauriClient implements UpdraftClient {
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
