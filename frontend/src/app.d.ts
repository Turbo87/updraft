// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    // interface PageState {}
    // interface Platform {}
  }

  interface Window {
    /** Test hook exposed only in `?testMode=1` (see docs/design/testing.md). */
    updraftTest?: { map: import('maplibre-gl').Map };
  }
}

export {};
