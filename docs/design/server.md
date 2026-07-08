# The axum Server

The `updraft-server` binary exposes the same Rust core over HTTP:

- **REST** for request/response interactions (queries, one-shot commands, file management),
- a **server-push stream** for continuous state-change notifications.

It can run **headless**, driven purely over its HTTP API with no user interface of its own. In this mode the system is inspected and controlled entirely through those transports, which suits machine-to-machine integration and automated testing.

It can also optionally serve the frontend's static assets, so any browser becomes its display. A soaring computer on a Raspberry-Pi-class device in the panel can run this way, and during development it is the fastest loop: run the server, open the frontend in a browser, no native build required. It is also the Playwright test target (see [testing.md](testing.md)).

Crucially, the Tauri IPC bridge and the axum server are two thin transports around the _same_ core API. Feature code never needs to know which one is in use.

## Bulk Data Routes

The server exposes the bulk geodata path (tiles, GeoJSON overlays) as native HTTP routes, per [core.md](core.md). These routes are part of the authenticated surface, see below.

## Security Model

Loopback is _not_ inherently safe: any website the user visits can fire requests at `127.0.0.1` (drive-by localhost, DNS rebinding). Defense follows the pattern Vite adopted after CVE-2025-24010:

- validate the `Host` header against an allowed-hosts list (default: localhost/loopback only),
- validate `Origin` on stream upgrades,
- keep CORS strict,
- require a session token on **all** routes: commands, the state stream, and the bulk data endpoints (`/tiles` and `/geojson` leak position and track). The served frontend receives the token at page load.

Binding to a non-loopback address additionally requires a configured password (login yields a session token). The server refuses to start non-loopback without one.

## Open Questions

- **WebSocket vs SSE** for the state-change stream: to be decided by whichever works best in practice.
