// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
import type { Pathname } from '$app/types';

declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    interface PageData {
      /** Dialog screen title. A getter so it re-resolves on locale change. */
      title?: () => string;
      /** Parent URL to step up to, or `null` at a flow root. */
      back?: Pathname | null;
    }
    // interface PageState {}
    // interface Platform {}
  }
}

export {};
