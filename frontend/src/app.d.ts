// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
  /* eslint-disable prefer-let/prefer-let -- Vite replaces these immutable build constants. */
  const __BUILD_COMMIT_SHA__: string | undefined;
  const __BUILD_TIMESTAMP__: string;
  /* eslint-enable prefer-let/prefer-let */

  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    // interface PageState {}
    // interface Platform {}
  }
}

export {};
