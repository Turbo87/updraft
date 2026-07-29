# Persisted locale setting

## Context

Updraft currently renders its language switcher over the flight view and
persists the selected locale in frontend `localStorage`. This slice moves the
setting across the existing Tauri boundary so the core owns the active setting
and the shell persists it.

The slice contains only the locale setting. It does not define a hierarchy or
placeholder model for settings that do not exist yet.

## Settings model

The core owns one `Settings` value:

```rust
enum Locale {
    En,
    De,
}

#[derive(Default)]
struct Settings {
    locale: Option<Locale>,
}
```

`Locale` serializes as the lowercase language codes `en` and `de`. `Settings`
is part of `CoreConfig` so every core instance receives its initial settings
explicitly.

`None` means that the user has not chosen an explicit language. The frontend
then resolves the browser or device language against the supported locales and
falls back to English. The settings screen still presents only English and
German. Once the user chooses either language, the core stores it explicitly.
There is no control for returning to automatic selection in this slice.

There is one application-wide settings value. No user or profile identity is
part of the model.

## Storage and startup

The shell stores a plain JSON file at
`app.path().app_config_dir()?.join("settings.json")`. Tauri resolves this to the
platform's private configuration directory for the `aero.updraft` application,
including the debug application identifier on Android debug builds.

An explicit German selection has this representation:

```json
{
  "locale": "de"
}
```

On startup, the shell loads the file before constructing the core and supplies
the resulting `Settings` as initial state. A missing file is the normal
first-run state and produces `Settings::default()`. An unreadable or malformed
file produces a warning and also uses defaults without modifying the file.
Startup does not create a settings file for the default value.

The existing frontend value is not migrated from `localStorage`.

## Core and IPC flow

The boundary gains these values:

- `Input::SetLocale(Locale)`
- `Topic::Settings(Settings)`
- `Effect::PersistSettings(Settings)`
- A Tauri `set_locale` command
- `UpdraftClient.setLocale(locale)` in the frontend client abstraction

The shell retains the settings path and persistence mechanism, not a second
mutable settings value. After startup, the core is authoritative.

`Core::topics()` includes the current settings so a new or recreated webview
receives them through the existing subscription channel. Generated TypeScript
types continue to define the topic payload.

When `Input::SetLocale` selects a different explicit locale, the core updates
its settings immediately and emits both the complete settings topic and a
persistence effect containing the complete settings snapshot. Selecting the
already active explicit locale produces no effects.

The `set_locale` command reports that the input was accepted for processing. It
does not wait for disk persistence.

## Background persistence

One shell-owned worker processes settings snapshots in order. For each
snapshot it:

1. Creates the application configuration directory if necessary.
2. Creates a temporary file next to `settings.json`.
3. Streams the JSON representation directly into the temporary file without
   first building a complete string or byte buffer.
4. Flushes the file.
5. Atomically replaces `settings.json`.

Serial processing prevents rapid changes from being persisted out of order.
Writing beside the destination allows the final rename to leave either the
previous complete file or the new complete file after interruption.

Persistence is intentionally asynchronous. A process exit immediately after a
change can lose the newest selection. A write failure logs a warning and does
not roll back the active locale. The shell does not report persistence status
to the core or frontend in this slice. A later user change replaces an invalid
settings file with a valid snapshot.

## Frontend

The locale switcher is removed from the flight view and replaced by a plain
text link to `/settings`. The settings route contains:

- A Settings heading.
- A top-level Language fieldset.
- English and German radio choices with the current circular flag icons and
  visible text labels.
- A plain link back to the flight view.

There are no empty categories or placeholders for later settings.

A settings store applies complete `Topic::Settings` values. An explicit locale
from the topic becomes the active Paraglide locale. With no explicit locale,
Paraglide resolves `preferredLanguage` and then `baseLocale`, which preserves
the current device-language behavior and English fallback.

Selecting a radio choice changes the native radio selection immediately and
calls `UpdraftClient.setLocale()`. Translations and document language still
change only when the resulting settings topic arrives. If the command fails,
the frontend logs the error and does not actively roll back the optimistic
radio selection in this slice.

Paraglide no longer uses its `localStorage` strategy, and its frontend locale
helper no longer persists selections. The fake client implements
`setLocale()` and emits the corresponding settings topic so browser
development follows the same interaction shape.

## Testing and acceptance

Core tests cover default settings, current settings on subscription, the topic
and persistence effects from a locale change, and the no-op for a repeated
explicit locale.

Shell tests use temporary directories to cover a missing file, a valid file, a
malformed file with its warning, directory and file creation, and replacement
of an earlier snapshot with valid settings JSON.

Frontend tests cover the two text-and-flag choices, optimistic radio selection,
supported and unsupported browser-language resolution, updates from explicit
settings topics, the `setLocale()` interaction through the fake client, and
navigation between the flight view and settings route.

Final acceptance runs in a real Tauri build:

1. Change the language and confirm that the selected control changes
   immediately, then the translated text and document language follow without
   visible flicker.
2. Restart the application and confirm the explicit locale remains active.
3. Confirm `settings.json` exists in the resolved application configuration
   directory.
4. Confirm browser-only development uses the same topic-driven interaction.
