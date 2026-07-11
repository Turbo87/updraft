// Host-aware logging for the frontend.
//
// The same frontend runs in two environments: inside the Tauri webview (where
// the native `tracing` subscriber owns the file / logcat / oslog outputs), and
// in a plain browser talking to the axum server (where there is no native side).
// This wrapper hides that difference: under Tauri it forwards each record to the
// native `frontend_log` command so webview diagnostics join the shared log
// stream; in a browser it falls back to the matching `console` method.
//
// Frontend logs are diagnostics only. Nothing safety-critical depends on them —
// warnings, audio, and IGC logging all live natively (see docs/design/tauri.md).

import { invoke, isTauri } from '@tauri-apps/api/core';

type Level = 'error' | 'warn' | 'info' | 'debug' | 'trace';

// Resolved once: `isTauri()` reads a global the Tauri runtime injects, which is
// absent in a normal browser build served by the server.
const underTauri = isTauri();

const consoleMethod: Record<Level, (...args: unknown[]) => void> = {
  error: console.error,
  warn: console.warn,
  info: console.info,
  debug: console.debug,
  trace: console.debug,
};

function emit(level: Level, message: string, location?: string): void {
  if (underTauri) {
    // Fire-and-forget: a failed log must never break the UI. The command lives
    // in tauri/src/lib.rs and re-emits at the matching `tracing` level.
    void invoke('frontend_log', { level, message, location }).catch(() => {});
  } else {
    const method = consoleMethod[level];
    if (location) {
      method(`[${location}] ${message}`);
    } else {
      method(message);
    }
  }
}

/**
 * Diagnostic logger shared by every frontend surface.
 *
 * `location` is an optional free-form tag for the call site (a component or
 * module name) that is preserved as a structured field natively and prefixed to
 * the message in the browser.
 */
export const log = {
  error: (message: string, location?: string) => emit('error', message, location),
  warn: (message: string, location?: string) => emit('warn', message, location),
  info: (message: string, location?: string) => emit('info', message, location),
  debug: (message: string, location?: string) => emit('debug', message, location),
  trace: (message: string, location?: string) => emit('trace', message, location),
};
