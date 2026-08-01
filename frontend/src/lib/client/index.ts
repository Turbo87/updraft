import type { Locale } from '$lib/protocol/generated/Locale';
import type { Topic } from '$lib/protocol/generated/Topic';
import type { UnitSettings } from '$lib/protocol/generated/UnitSettings';

export type TopicListener = (topic: Topic) => void;

/**
 * The only boundary between the frontend and the Rust shell.
 *
 * Components never import an implementation of this. The layout receives
 * one, so tests and browser-only development can substitute the fake.
 */
export interface UpdraftClient {
  /**
   * Starts delivering topics to `onTopic`.
   *
   * The returned function stops local delivery. It does not tell the Rust
   * side to stop sending: the driver prunes a subscriber only when a send
   * to it fails, which happens when the webview goes away. That is enough
   * while the layout owns the only subscription and never unmounts.
   */
  subscribe(onTopic: TopicListener): () => void;
  setLocale(locale: Locale): Promise<void>;
  /** Replaces all display-unit selections. */
  setUnits(units: UnitSettings): Promise<void>;
}
