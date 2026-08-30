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

const TEST_STYLE: StyleSpecification = {
  version: 8,
  sources: {},
  layers: [],
};

/** Returns a blank style in test mode or Positron with sprites at the specified origin. */
export function getBasemapStyle(testMode: boolean, origin: string): StyleSpecification {
  if (testMode) return TEST_STYLE;

  let sprite = `${origin}/basemap/sprites/ofm`;
  return { ...POSITRON_STYLE, sprite };
}
