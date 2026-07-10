// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    interface PageState {
      /** Depth/opener of the current dialog history entry, restored on back/forward. */
      dialog?: { depth: number; hasOpener: boolean };
    }
    // interface Platform {}
  }
}

export {};
