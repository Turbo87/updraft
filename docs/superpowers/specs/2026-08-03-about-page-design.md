# About Page

## Purpose

Updraft does not have a page that identifies its source or build. The flight
view also uses MapLibre's attribution control to show map-data credits. The
control occupies map space that is useful for flight information.

This design adds an About page under Settings. The page links to the source
repository, identifies the frontend build, and shows the map source
attributions that are available when the page opens. It replaces the
attribution control on the flight view for the current development phase.

## Scope

This design includes these functions:

- Add an About entry to the Settings menu.
- Add the `/settings/about` route.
- Link to the Updraft GitHub repository.
- Show an abbreviated Git commit SHA that links to the exact commit.
- Show the localized build date and time.
- Show the current MapLibre source attributions when they are available.
- Render attribution text and links without rendering arbitrary HTML.
- Remove MapLibre's attribution control from the flight view.

This design does not include these functions:

- An application version.
- A general application description.
- Application or third-party software license information.
- Privacy or other legal information.
- A layer switcher.
- Attribution updates while the About page is open.
- Rust, Tauri, protocol, storage, or persistence changes.

## Relationship to the Settings route specification

This specification extends the route structure in
[`2026-08-03-settings-route-structure.md`](2026-08-03-settings-route-structure.md).
It supersedes only the list of routes, Settings menu entries, and related
navigation tests in that specification. All other requirements remain active.

The Settings flow adds this route:

- `/settings/about` shows source, build, and map-data information.

The Settings menu adds **About** after the existing settings categories. The
About page starts with a back link to `/settings`. The link appears before the
page heading in document order, as it does on the other settings subpages.

## Page content

The page has an **About** heading and these sections in order:

1. **Source** contains a link to
   `https://github.com/Turbo87/updraft`.
2. **Build** contains the Git commit and build date and time.
3. **Data credits** contains the available source-provided map attributions.

The heading, section labels, repository-link label, and build fallback text
are localized. Source-provided attribution text is not translated.

## Build information

The Vite configuration defines two frontend build values:

- The full Git commit SHA from `git rev-parse HEAD`.
- An ISO 8601 timestamp created once when the Vite configuration starts.

The frontend displays the first seven characters of a known commit SHA. The
abbreviated value links to
`https://github.com/Turbo87/updraft/commit/<full-sha>`. The page does not show
the full SHA as text.

Failure to run Git does not fail the frontend build. An unavailable commit
produces the localized text **Unknown version** without a link. The page still
shows the build timestamp.

The timestamp represents one fixed build instant. The page formats that
instant with the current application locale and the device time zone. It does
not show the raw ISO 8601 value.

## Attribution snapshot

The root layout keeps the flight view and MapLibre instance mounted behind
route screens. The About page reads the map through
`getAppContext().mapState.map` when the page component initializes.

The page takes one attribution snapshot. It does not wait for the map, retry,
subscribe to MapLibre events, or refresh the snapshot while the page is open.
A later source or metadata change does not update the page.

When a map is available, the page:

1. Reads the source IDs from `map.getStyle().sources`.
2. Reads each current source through `map.getSource(sourceId)`.
3. Collects each non-empty `source.attribution` value.
4. Trims the values and removes exact duplicates while keeping the first
   source order.

A missing map, missing style, or empty attribution result omits the complete
Data credits section. The page does not show a loading state, placeholder, or
static fallback. The user can leave and open the page again to take another
snapshot.

The snapshot contains source-provided attributions only. It does not add
MapLibre's default project credit or other custom attribution.

## Safe attribution rendering

A source attribution is an HTML string. The page parses each string with
`DOMParser` and converts it to a small display model before Svelte renders it.

The parser applies these rules:

- Preserve text nodes.
- Preserve an anchor as a link only when its resolved URL uses `http:` or
  `https:`.
- Preserve the visible text of an anchor whose URL is absent or rejected, but
  do not create a link.
- Ignore every attribute except an accepted anchor `href`.
- Use the descendant text of other harmless elements.
- Discard scripts, styles, images, and other active or non-text content.

Svelte renders the resulting text and link values through normal template
markup. The page does not use `{@html}`. The implementation does not add an
HTML-sanitization dependency and does not import MapLibre's private DOM
utilities.

## Attribution placement

The MapLibre component disables the built-in attribution control. The About
page is the only location for map-data credits in this design.

## Automated tests

Implementation follows red-green-refactor.

Frontend tests cover these behaviors:

- A known commit displays seven characters and links with the full SHA.
- An unavailable commit displays **Unknown version** without a link.
- A fixed timestamp uses the current locale and device time zone.
- Attribution parsing preserves text and HTTP or HTTPS links.
- Attribution parsing rejects unsafe URLs and active content.
- Empty values and exact duplicate attributions are omitted.
- A missing map or missing attribution omits the Data credits section.

End-to-end tests cover these behaviors:

- The Settings menu links to `/settings/about`.
- The About page starts with a back link to `/settings`.
- The repository link uses the canonical GitHub URL.
- The current build shows the expected abbreviated SHA and exact commit URL.
- A source with attribution is added before the About page opens, and the page
  shows its safe text and links.
- The page does not show Data credits when the current sources have no
  attribution.
- The flight view does not contain MapLibre's attribution control.

The tests do not wait for attribution changes after the About page opens. No
Rust, Tauri, protocol, persistence, or restart test is necessary.

## Acceptance criteria

This feature is complete when all of these statements are true:

- Settings provides an About route with the standard back-link structure.
- The page links to the canonical Updraft repository.
- The page shows the abbreviated build commit or **Unknown version**.
- A known commit links to the exact GitHub commit.
- The page shows a localized build date and time.
- The page takes one snapshot of current source-provided attributions.
- The page safely preserves attribution text and HTTP or HTTPS links.
- The Data credits section is absent when the snapshot has no credits.
- The flight view no longer shows MapLibre's attribution control.
- The implementation does not add an application version or backend changes.
