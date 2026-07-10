# The axum Server

The axum server is the **single transport** for the whole system. It exposes the Rust core over HTTP:

- **REST** for request/response interactions (queries, commands, file management),
- **one multiplexed SSE stream** for the state stream (snapshot, then changes, see [core.md](core.md)),
- the **bulk geodata routes** (tiles, GeoJSON overlays, track tail, see [core.md](core.md)).

It runs in two hostings from the same code: as the standalone `updraft-server` binary, and embedded in the Tauri shell, bound to loopback (see [tauri.md](tauri.md)). Same routes, same auth, same stream semantics everywhere — there is no second transport, so the Playwright suite exercises the transport that ships in the native apps.

It can run **headless**, driven purely over its HTTP API with no user interface of its own. In this mode the system is inspected and controlled entirely through those transports, which suits machine-to-machine integration and automated testing.

It can also serve the frontend's static assets, so any browser becomes its display. A soaring computer on a Raspberry-Pi-class device in the panel can run this way, and during development it is the fastest loop: run the server, open the frontend in a browser, no native build required. It is also the Playwright test target (see [testing.md](testing.md)).

## The State Stream: one SSE connection

Server push uses **SSE** — decided, not open: the stream is strictly server→client (commands travel as ordinary POSTs), EventSource reconnects automatically, and reconnection composes with "subscribe delivers a fresh snapshot" ([core.md](core.md)) into reconnect handling that needs no client code. WebSocket would buy binary framing this design doesn't use, at the price of hand-rolled reconnect and heartbeat logic; revisit only on a measured need.

Three constraints shape the implementation:

- **Exactly one stream per client, multiplexing all change groups.** Browsers cap HTTP/1.1 at ~6 connections per origin, cleartext loopback can never negotiate HTTP/2, and each EventSource holds its connection forever — per-topic streams would starve MapLibre's tile fetches against the same origin.
- **Keep-alive comments are mandatory** (axum's `Sse` sends none by default): an idle stream — app open on the ground, nothing changing — must not look dead to middleboxes or power management, and a half-dead TCP connection often surfaces no error to EventSource.
- **EventSource cannot set request headers**, so the stream authenticates via a query-parameter session token or a cookie scoped to the server origin.

The client treats stream errors and silence as staleness: the frontend surfaces data age rather than freezing silently (see [frontend.md](frontend.md)).

## Bulk Data Routes

The server exposes the bulk geodata path (tiles, GeoJSON overlays, the `?since=<seq>` track tail) as native HTTP routes, per [core.md](core.md). These routes are part of the authenticated surface, see below.

## Security Model

Loopback is _not_ inherently safe: any website the user visits can fire requests at `127.0.0.1` (drive-by localhost, DNS rebinding), and any process on the device can connect to a loopback listener. Defense follows the pattern Vite adopted after CVE-2025-24010:

- validate the `Host` header against an allowed-hosts list (default: localhost/loopback only),
- validate `Origin` on state-stream subscriptions and keep CORS strict (in the embedded hybrid asset shape, the allowlist carries exactly the webview origin, see [tauri.md](tauri.md)),
- require a session token on **all** routes: commands, the state stream, and the bulk data endpoints (`/tiles` and `/geojson` leak position and track).

**Token distribution differs per hosting.** Embedded: the shell injects the token into the webview at startup (initialization script or one-shot URL nonce) — the token is never obtainable by merely fetching the page, so another local process can connect to the port but cannot read anything. Standalone on loopback: the server prints a tokened URL on startup (Jupyter-style) that the user opens. Binding to a non-loopback address additionally requires a configured password (login yields a session token); the server refuses to start non-loopback without one.

**Ports.** The standalone binary defaults to a fixed port for dev and bookmarking convenience. The embedded instance binds an ephemeral port (`:0`) — no collision policy needed — and hands the resulting origin to the webview; this is safe because nothing durable lives in origin-keyed browser storage (see [frontend.md](frontend.md)).
