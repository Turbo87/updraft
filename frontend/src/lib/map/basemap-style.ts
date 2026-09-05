import type { StyleSpecification } from 'maplibre-gl';

import { convertFileSrc } from '@tauri-apps/api/core';

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
 *
 * Remove `sources.ne2_shaded` after each refresh. No layer uses this source.
 */
const POSITRON_STYLE = positron as StyleSpecification;

export const BASEMAP_MIN_ZOOM = 6;

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

/** Returns a blank test style or Positron with local Enroute tiles and bundled assets. */
export function getBasemapStyle(testMode: boolean, origin: string): StyleSpecification {
  let overlaySprite = { id: 'updraft-sdf', url: `${origin}/sprites/updraft-sdf` };
  if (testMode)
    return {
      ...TEST_STYLE,
      glyphs: `${origin}/basemap/fonts/{fontstack}/{range}.pbf`,
      sprite: [overlaySprite],
    };

  let style = structuredClone(POSITRON_STYLE);
  let basemapUrl = convertFileSrc('basemap', 'updraft');
  style.sources.openmaptiles = {
    type: 'vector',
    tiles: [`${basemapUrl}/{z}/{x}/{y}.pbf`],
    minzoom: BASEMAP_MIN_ZOOM,
    maxzoom: 10,
    attribution:
      '<a href="https://www.openstreetmap.org/copyright">© OpenStreetMap contributors</a> | ' +
      '<a href="https://akaflieg-freiburg.github.io/enroute/">Enroute Flight Navigation</a> | ' +
      '<a href="https://www.akaflieg-freiburg.de/">Akaflieg Freiburg</a>',
  };
  style.glyphs = `${origin}/basemap/fonts/{fontstack}/{range}.pbf`;
  style.sprite = [{ id: 'default', url: `${origin}/basemap/sprites/ofm` }, overlaySprite];

  for (let layer of style.layers) {
    if ('layout' in layer && layer.layout && 'text-font' in layer.layout) {
      layer.layout['text-font'] = (layer.layout['text-font'] as string[]).flatMap(
        (fontStack) => FONT_REPLACEMENTS[fontStack] ?? [fontStack],
      );
    }
  }

  return style;
}
