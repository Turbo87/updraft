# Multiple Clients

The pilot's app runs the core and is the **primary**. A copilot can connect a **secondary client** from another phone or tablet over the local network. Both clients use the axum server that is already embedded in the pilot's app (see [server.md](server.md) and [tauri.md](tauri.md)). Network sharing requires a password. A _device_ means a connected instrument, never a phone or tablet running the frontend (see [devices.md](devices.md)).

- The primary's core remains the trusted source for shared flight state. Each client keeps its own temporary presentation state, such as its viewport and open dialogs. Saved layouts belong to a display profile, so two displays may use different layouts.
- A secondary client is "just another frontend": it authenticates, receives a snapshot followed by ordered changes, and may send commands, exactly like the local UI does.
- Roles and permissions decide what a secondary client may change (e.g. the copilot can edit the task and acknowledge warnings, but the primary's settings stay local).
- The primary client sends its current map viewport as a temporary OGN area-of-interest request (see [traffic.md](traffic.md)). Secondary clients see the shared traffic table but do not control traffic downloads.

Because every UI is already a message-driven client of the core, this feature falls out of the architecture rather than being bolted on. The same mechanism covers related features such as repeater displays and two-seat operation.

## Open Questions

- **Version skew between clients:** the ts-rs golden-file check (see [core.md](core.md)) guards a single build, but a secondary client may run a different app version than the primary. Likely answer: a protocol version handshake at connect time that warns or refuses on mismatch.
