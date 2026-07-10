# The axum Server

The axum server is the **single transport** for the whole system. It exposes the Rust core over HTTP:

- **REST** for request/response interactions (queries, commands, file management),
- **SSE streams** for per-topic state-change notifications,
- the **bulk geodata routes** (tiles, GeoJSON overlays, see [core.md](core.md)).

It runs in two hostings from the same code: as the standalone `updraft-server` binary, and embedded in the Tauri shell, bound to loopback, serving the webview (see [tauri.md](tauri.md)). Same routes, same auth, same stream semantics everywhere — there is no second transport, so the Playwright suite exercises byte-for-byte the transport that ships in the native apps.

It can run **headless**, driven purely over its HTTP API with no user interface of its own. In this mode the system is inspected and controlled entirely through those transports, which suits machine-to-machine integration and automated testing.

It can also optionally serve the frontend's static assets, so any browser becomes its display. A soaring computer on a Raspberry-Pi-class device in the panel can run this way, and during development it is the fastest loop: run the server, open the frontend in a browser, no native build required. It is also the Playwright test target (see [testing.md](testing.md)).

## State Streams: SSE

Server push uses **SSE**, not WebSocket. The subscription streams are strictly server→client (commands travel as ordinary POSTs), SSE reconnects automatically — which composes with "every topic delivers its current state on subscribe" ([core.md](core.md)) into reconnect handling that needs no code — and it is plain HTTP: friendly to proxies and HTTP/2 multiplexing, and free of the secure-context restrictions webviews apply to `ws://`. The topic abstraction leaves room for a binary WebSocket channel later if a high-rate topic (live vario) ever demands one.

## Bulk Data Routes

The server exposes the bulk geodata path (tiles, GeoJSON overlays) as native HTTP routes, per [core.md](core.md). These routes are part of the authenticated surface, see below.

## Security Model

Loopback is _not_ inherently safe: any website the user visits can fire requests at `127.0.0.1` (drive-by localhost, DNS rebinding), and any process on the device can connect to a loopback listener — the reason Tauri discourages serving unauthenticated app assets over localhost. Defense follows the pattern Vite adopted after CVE-2025-24010:

- validate the `Host` header against an allowed-hosts list (default: localhost/loopback only),
- validate `Origin` on stream subscriptions,
- keep CORS strict,
- require a session token on **all** routes: commands, the state streams, and the bulk data endpoints (`/tiles` and `/geojson` leak position and track). The served frontend receives the token at page load, which also covers the embedded case: another local app can connect to the port but cannot read anything.

Binding to a non-loopback address additionally requires a configured password (login yields a session token). The server refuses to start non-loopback without one.

## Open Questions

- **Port strategy for the embedded instance:** fixed port vs ephemeral port handed to the webview at startup, and behavior on port collision.
