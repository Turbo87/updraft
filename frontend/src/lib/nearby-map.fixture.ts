import type { Map as MapLibreMap } from 'maplibre-gl';

export function previewMap(features: unknown[]): MapLibreMap {
  return {
    on() {},
    off() {},
    isStyleLoaded: () => true,
    isSourceLoaded: () => true,
    getSource: () => ({}),
    getLayer: () => ({}),
    project: () => ({ x: 0, y: 0 }),
    queryRenderedFeatures: () => features,
  } as unknown as MapLibreMap;
}
