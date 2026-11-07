# Brand assets

This directory contains the source artwork for the Updraft logo. The
application icons and the logo are derived from these files.

## Mark

The mark is a capital U. Its right stem rises above the left stem and ends in
an arrowhead. `mark.svg` contains the mark alone in `#0A72D9`.

The icons centre the U body horizontally, so the arrowhead extends to the
right. They centre the full shape, including the arrowhead, vertically.

## Icon

The icon shows the mark in front of a sky. Two ridge silhouettes fill the
bottom of the sky. Thin streamlines rise from the back ridge. A soft drop
shadow and a bevel, light on the top edges and dark on the bottom edges, lift
the mark above the sky. The monochrome and notification icons are flat.

| File                            | Use                                                 |
| ------------------------------- | --------------------------------------------------- |
| `icon-foreground.svg`           | Android adaptive icon foreground, 108 x 108 units   |
| `icon-background.svg`           | Android adaptive icon background, 108 x 108 units   |
| `icon-monochrome.svg`           | Android themed icon layer, 108 x 108 units          |
| `icon-android-legacy.svg`       | Android 7 launcher icon, 38 dp square keyline       |
| `icon-android-legacy-round.svg` | Android 7 round launcher icon, 44 dp circle keyline |
| `icon-macos.svg`                | macOS icon on Apple's app icon grid                 |
| `icon-square.svg`               | Rounded-square icon, source for Windows and Linux   |
| `icon-full-bleed.svg`           | Full-bleed square, for example for store listings   |
| `icon-circle.svg`               | Full-bleed artwork in a circle, used in the logo    |
| `notification.svg`              | Android status bar icon, 24 x 24 units              |

The adaptive icon foreground and the Android 7 icons in
`tauri/gen/android/app/src/main/res` are PNG renders at each density, because
vector drawables cannot blur the drop shadow. The other Android resources,
including the notification icon in
`libs/tauri_plugin_updraft/android/src/main/res`, are vector drawables of the
same geometry. `tauri icon` generates
the files in `tauri/icons` from a 1024 px render of `icon-square.svg`, except
`icon.icns`, which it generates from a 1024 px render of `icon-macos.svg`.

The platform files follow each platform's icon grid. The macOS icon is an
824 px continuous-corner tile on a 1024 px canvas with a soft drop shadow.
The Android 7 icons follow the Material product icon keylines on the 48 dp
grid. The adaptive icon keeps the mark inside the 66 dp safe zone.

## Logo

The logo is the circle icon followed by the name "Updraft" in outlines of the
Outfit typeface. The horizontal logo centres the icon on the cap height of the
name. The stacked logo puts the icon above the name. Only the colour of the
name changes between themes.

| File                         | Use                                        |
| ---------------------------- | ------------------------------------------ |
| `logo-horizontal.svg`        | Horizontal logo for light backgrounds      |
| `logo-horizontal-dark.svg`   | Horizontal logo for dark backgrounds       |
| `logo-stacked.svg`           | Stacked logo for light backgrounds         |
| `logo-stacked-dark.svg`      | Stacked logo for dark backgrounds          |

The About screen shows the horizontal logo. `frontend/src/lib/Logo.svelte`
contains the same name path, and `frontend/src/lib/assets/icon-circle.svg` is
a copy of `icon-circle.svg`. The repository README shows the stacked logo.

## Colours

| Role | Light     | Dark      |
| ---- | --------- | --------- |
| Name | `#17202B` | `#F2F5F8` |

The icon does not change with the theme. Its sky is a vertical gradient from
`#0F3A6E` at the bottom through `#12519A` to `#2A86E0` at the top. The mark is
a vertical gradient from `#C6DDF4` at the bottom through `#EAF3FC` to white at
the arrowhead.
