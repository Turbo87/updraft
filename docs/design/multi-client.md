# Multiple Clients

The architecture supports a **primary/secondary setup**: the pilot's app instance runs the core (the primary), and a copilot can connect a **secondary client** from another phone or tablet over the local network via the axum server — the same server the shell already embeds for its own webview (see [server.md](server.md) and [tauri.md](tauri.md)); sharing means binding it beyond loopback, gated by the password login. A _device_ always means a connected instrument, never one of these clients (see [devices.md](devices.md)).

- The primary's core remains the single source of authoritative domain state. Each client keeps its own presentation state (viewport, dialogs, layout).
- A secondary client is "just another frontend": it authenticates, receives a snapshot followed by ordered changes, and may send commands, exactly like the local UI does.
- Roles and permissions decide what a secondary client may change (e.g. the copilot can edit the task and acknowledge warnings, but the primary's settings stay local).
- The primary client's map viewport supplies the OGN area of interest (see [traffic.md](traffic.md)); secondary clients observe the shared traffic table but do not steer traffic acquisition.

Because every UI is already a message-driven client of the core, this feature falls out of the architecture rather than being bolted on. The same mechanism covers related features such as repeater displays and two-seat operation.

## Open Questions

- **Version skew between clients:** the ts-rs golden-file check (see [core.md](core.md)) guards a single build, but a secondary client may run a different app version than the primary. Likely answer: a protocol version handshake at connect time that warns or refuses on mismatch.
