# Brand assets

This directory contains the source artwork for the Updraft logo. The
application icons and the in-app wordmark are derived from these files.

## Mark

The mark is a capital U. Its right stem rises above the left stem and ends in
an arrowhead. `mark.svg` contains the mark alone in the wordmark mark colour.

The icons centre the U body horizontally, so the arrowhead extends to the
right. They centre the full shape, including the arrowhead, vertically.

## Icon

The icon shows the mark in front of a sky. Two ridge silhouettes fill the
bottom of the sky. Thin streamlines rise from the back ridge.

| File                  | Use                                               |
| --------------------- | ------------------------------------------------- |
| `icon-foreground.svg` | Android adaptive icon foreground, 108 x 108 units |
| `icon-background.svg` | Android adaptive icon background, 108 x 108 units |
| `icon-monochrome.svg` | Android themed icon layer, 108 x 108 units        |
| `icon-square.svg`     | Rounded-square icon, source for desktop icons     |
| `icon-round.svg`      | Round icon for launchers without adaptive icons   |
| `icon-full-bleed.svg` | Full-bleed square, for example for store listings |
| `notification.svg`    | Android status bar icon, 24 x 24 units            |

The Android resources in `tauri/gen/android/app/src/main/res` and
`libs/tauri_plugin_updraft/android/src/main/res` are vector drawables of the
same geometry. `tauri icon` generates the files in `tauri/icons` from a
1024 px render of `icon-square.svg`.

## Wordmark

The wordmark is the mark followed by "pdraft" in outlines of the Outfit
typeface. `wordmark.svg` is for light backgrounds, `wordmark-dark.svg` is for
dark backgrounds, and `wordmark-single.svg` uses the ink colour only.
`frontend/src/lib/Wordmark.svelte` contains the same paths.

## Colours

| Role                  | Light     | Dark      |
| --------------------- | --------- | --------- |
| Wordmark mark         | `#0A72D9` | `#2B8FF5` |
| Wordmark text and ink | `#17202B` | `#F2F5F8` |

The icon does not change with the theme. Its sky is a vertical gradient from
`#0F3A6E` at the bottom through `#12519A` to `#2A86E0` at the top. The mark is
a vertical gradient from `#C6DDF4` at the bottom through `#EAF3FC` to white at
the arrowhead.
