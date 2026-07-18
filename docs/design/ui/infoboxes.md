# Infoboxes

Infoboxes are configurable cells that show flight information or provide a
small, focused control. Their content is not limited to navigation. Infoboxes
sit in a dock that reserves space at an edge of the map and are grouped into
ordered pages.

Portrait initially places the dock along the bottom. Landscape places it along
the right edge to preserve vertical map space. The first implementation may use
fixed slots. The target design uses a snap grid with movable and resizable
infoboxes.

## Pages and Gestures

Pages have a fixed linear order and do not wrap. This gives the pilot a stable
mental model of where each page is located.

The page gesture follows the long axis of the dock:

- In portrait, swipe left for the next page and right for the previous page.
- In landscape, swipe up for the next page and down for the previous page.

The gesture must begin inside the infobox dock. Gestures beginning on the map
remain map gestures. After a successful swipe, a small non-clickable indicator
such as `Thermal · 2/4` appears for about two seconds and fades. The Flight View
has no permanent page arrows, page button, or clickable page indicator.

Hardware-button page switching may be added later through configurable input
bindings.

## Thermal Page

Thermal is an ordinary customizable page with automatic behavior. It is enabled
by default and activates when circling begins unless the user disabled the page.
Disabling Thermal also disables its automatic activation.

While circling, the pilot may swipe away and later return to Thermal. The app
does not force Thermal back during the same circling episode. When circling ends,
the page that was active before circling is restored only if Thermal is still
visible. If the pilot is viewing another page, that manual selection remains.

## Quick Interaction

Tapping an infobox opens its quick panel. The panel contains any operational
control or details for that value and a visible **Change infobox** action. Change
opens a searchable, categorized value picker.

Replacement applies immediately. It preserves the infobox position, size,
appearance, and page. Because the saved layout is shared across orientations,
the replacement appears in portrait and landscape. There is no long-press
interaction.

## Page Management and Layout Editor

**Main Menu → Display & Infoboxes** opens a list of every configured page,
including Thermal whether it is enabled or disabled. Selecting any page opens
its layout editor, regardless of which page is currently active on the Flight
Deck. Page order and other page-level actions are managed from this list.

The editor supports moving, resizing, adding, removing, duplicating, and styling
infoboxes on the selected page. Editing remains available in flight.

During editing, the grid is visible and automatic page changes pause. Warnings
remain visible. The editor has explicit Done and Cancel actions. After the pilot
returns to the Flight View, the app reevaluates the current flight mode and may
activate Thermal if a new circling episode requires it.

One logical layout applies to portrait and landscape. A prototype must compare
ordered reflow, a normalized shared grid, and common dock geometry before the
resizable editor commits to one mapping. The comparison should emphasize
predictability after rotation, map area, wide-infobox readability, in-flight
editing, persistence complexity, and testability.
