# Settings Route Structure

## Purpose

The Settings screen contains several independent categories. One long page
makes navigation less clear as more categories become available. This design
makes `/settings` a menu. Each implemented category has a dedicated route.

This design changes only frontend routes and navigation. It does not change
settings ownership, persistence, client commands, validation, or control
behavior.

## Superseded specification sections

This specification supersedes only the route and page-structure details in
these sections:

- The **Frontend** section in
  [`2026-07-28-settings-design.md`](2026-07-28-settings-design.md).
- The **Settings screen** section in
  [`2026-08-01-configurable-units-design.md`](2026-08-01-configurable-units-design.md).
- The **Navigation and routes** section and the return routes in the **Add and
  edit flow** section in
  [`2026-08-01-devices-screen-design.md`](2026-08-01-devices-screen-design.md).
- The page-location details in the **Settings user interface** section in
  [`2026-08-02-openair-airspace-design.md`](2026-08-02-openair-airspace-design.md).

All other requirements in those specifications remain active.

## Route structure

The settings flow uses these routes:

- `/settings` shows the settings menu.
- `/settings/language` shows the language controls.
- `/settings/units` shows the unit controls.
- `/settings/airspace` shows the local airspace controls.
- `/settings/devices` shows the configured external devices.
- `/settings/devices/new` adds one external device.
- `/settings/devices/[deviceId]` edits one external device.

The old `/devices`, `/devices/new`, and `/devices/[deviceId]` routes do not
remain as aliases.

## Settings menu

The `/settings` route shows a Settings heading and links to the four settings
categories. It does not show the category controls. A separate link returns to
the flight view.

The menu uses the existing localized category names:

- Language.
- Units.
- Airspace.
- External devices.

## Settings subpages

Each settings subpage starts with a back link to `/settings`. The link appears
before the page heading and controls in document order.

The language page keeps the existing locale resolution and language controls.
The units page keeps the existing optimistic unit selection behavior. The
airspace page keeps the existing import, replace, and remove behavior. The
devices page keeps the existing device list and bonded-device query behavior.

## Device add and edit routes

The device add and edit routes start with a back link to `/settings/devices`.
A successful add, edit, or delete operation returns to `/settings/devices`.
An unknown device also provides a link to `/settings/devices`.

## Testing

End-to-end tests verify the four menu links and their exact routes. They verify
that each settings subpage starts with a back link to `/settings`. Component
tests verify the nested device add and edit links. Existing behavior tests
continue to cover language, units, airspace, and device controls.
