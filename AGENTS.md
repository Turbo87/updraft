# Agent Guide

Updraft is a cross-platform soaring flight computer. The current work targets an MVP for reliable situational awareness. Over time, Updraft can grow to include the main features that pilots expect from established soaring flight computers.

The current technology stack is:

- Rust for domain logic, parsers, and native application code.
- Tauri for the desktop and Android application shell.
- Kotlin for Android platform integration.
- SvelteKit, Svelte, and TypeScript for the frontend.
- MapLibre GL JS for maps.
- pnpm and Vite for frontend tooling.

This file contains project rules that agents often miss. Keep changes small, specific, and easy to review.

## Sources of truth

Use `docs/README.md` as the documentation entry point. Use `docs/architecture.md` for current system boundaries and `docs/roadmap.md` for delivery status.

Files under `docs/design/` and `docs/superpowers/specs/` are historical. Do not use them as current requirements. Report a missing or unclear current requirement instead of combining historical designs.

Current product documents define accepted intended behavior. Code and tests define implemented behavior. Treat prototypes and simulators as evidence, not as architecture or hardware-compatibility guarantees.

## Scope and design

Implement the smallest solution that meets the current requirement. Do not add general APIs, placeholder fields, wrapper abstractions, diagnostics, or configuration for possible future use.

Introduce one new concept at a time. Keep each commit independently understandable and reviewable.

## Tests

Test behavior at the layer that owns it. Do not repeat lower-layer parser or protocol tests in higher layers. Add integration tests only for behavior that crosses a boundary. Do not add another test when an existing end-to-end test proves the same behavior.

Prefer exact equality when the result is deterministic. Range assertions should be rare.

Use the `claims` macros for `Result`, `Option`, and relational assertions:

```rust
assert_ok!(result);
assert_some_eq!(value, expected);
assert_le!(actual, limit);
```

Do not replace them with less informative boolean assertions:

```rust
assert!(result.is_ok()); // Use assert_ok!(result).
assert_eq!(value, Some(expected)); // Use assert_some_eq!(value, expected).
assert!(actual <= limit); // Use assert_le!(actual, limit).
```

Use `std::assert_matches!()` for pattern assertions. Use `approx` when a floating-point tolerance is part of the requirement.

Use Insta snapshots for structured, multiline, serialized, or scenario output. Use direct assertions for small scalar values.

For Tauri command tests, deserialize through the same Tauri IPC path that production uses. Do not use a direct `serde_json` test as a substitute.

Use Storybook to present isolated component states. Do not duplicate component or end-to-end behavior tests in Storybook. Add visual-regression tests only when the project adopts them.

## Validation

Run the relevant checks for the changed area. Use `.github/workflows/` as the source of truth for complete validation. Report checks that you could not run.

## Logging

Write concise, self-contained event messages. State the subject and the event. Do not use a single word or a long prose paragraph.

Put the main event in the message. Add structured fields when filtering or correlation make them useful. Do not add fields only for a hypothetical consumer.

Use log levels consistently:

- `error` means that an operator must act.
- `warn` means that the program recovered from an unexpected or degraded state.
- `info` records normal high-level lifecycle events.
- `debug` records diagnostic details.
- `trace` records fine-grained or high-volume details.

Log raw transport payloads only at `trace`. Never log credentials or secrets.

## Technical writing

Use Simplified Technical English (ASD-STE100) for technical prose. This rule applies to agent output, documentation, code comments, commit messages, and pull request titles and descriptions. Use short active sentences. Put one idea in each sentence. Use exact terms. Remove unnecessary prose.

Do not apply ASD-STE100 to creative or marketing copy.

Document decisions, constraints, and behavior that could reasonably differ. Omit statements that only restate the implementation or describe unavoidable consequences.

Keep comments true in every commit. Update or remove a comment when the code changes the behavior that it describes.

Keep committed documentation self-contained and safe for public review. Do not rely on host-local paths, unavailable artifacts, local plans, or unpushed branches. Redact credentials, personal identifiers, and unique device identifiers.
