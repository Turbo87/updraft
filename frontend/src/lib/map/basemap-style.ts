import type { StyleSpecification } from 'maplibre-gl';

import positron from './style/positron.json';

/*
 * The Positron snapshot was retrieved from
 * https://tiles.openfreemap.org/styles/positron on 2026-08-29.
 *
 * Refresh it from the repository root with:
 *
 * curl --fail --location \
 *   https://tiles.openfreemap.org/styles/positron \
 *   --output frontend/src/lib/map/style/positron.json
 * pnpm prettier --write frontend/src/lib/map/style/positron.json
 */
const POSITRON_STYLE = positron as StyleSpecification;

/** The bundled font stack for map overlay text. */
export const FONT_REGULAR = ['Barlow Semi Condensed Medium', 'Noto Sans Medium'];

const FONT_BOLD = ['Barlow Semi Condensed Bold', 'Noto Sans Bold'];
const FONT_ITALIC = ['Barlow Semi Condensed Medium Italic', 'Noto Sans Medium Italic'];

const FONT_REPLACEMENTS: Record<string, string[]> = {
  'Noto Sans Regular': FONT_REGULAR,
  'Noto Sans Italic': FONT_ITALIC,
  'Noto Sans Bold': FONT_BOLD,
};

const TEST_STYLE: StyleSpecification = {
  version: 8,
  sources: {},
  layers: [],
};

/** Returns a blank style in test mode or Positron with bundled presentation assets. */
export function getBasemapStyle(testMode: boolean, origin: string): StyleSpecification {
  if (testMode) return TEST_STYLE;

  let style = structuredClone(POSITRON_STYLE);
  style.glyphs = `${origin}/basemap/fonts/{fontstack}/{range}.pbf`;
  style.sprite = `${origin}/basemap/sprites/ofm`;

  for (let layer of style.layers) {
    if ('layout' in layer && layer.layout && 'text-font' in layer.layout) {
      layer.layout['text-font'] = (layer.layout['text-font'] as string[]).flatMap(
        (fontStack) => FONT_REPLACEMENTS[fontStack] ?? [fontStack],
      );
    }
  }

  return style;
}
