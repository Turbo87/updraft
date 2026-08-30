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

/** Returns a blank style in test mode and the bundled Positron style otherwise. */
export function getBasemapStyle(testMode: boolean): StyleSpecification {
  return testMode ? TEST_STYLE : { ...POSITRON_STYLE };
}
