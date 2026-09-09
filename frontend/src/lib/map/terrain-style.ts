import type { Map } from 'maplibre-gl';

import { convertFileSrc } from '@tauri-apps/api/core';

export function refreshTerrain(map: Map, generation: number): void {
  let style = map.getStyle();
  let source = style.sources.terrain;
  if (!source || source.type !== 'raster-dem') return;
  let base = `${convertFileSrc('terrain', 'updraft')}/${generation}`;
  let url = `${base}/metadata.json`;
  if (source.url === url) return;
  for (let layer of style.layers) {
    if ('source' in layer && layer.source === 'terrain') map.removeLayer(layer.id);
  }
  map.removeSource('terrain');
  map.addSource('terrain', { ...source, url, tiles: [`${base}/{z}/{x}/{y}.webp`] });
  for (let i = style.layers.length - 1; i >= 0; i--) {
    let layer = style.layers[i];
    if ('source' in layer && layer.source === 'terrain')
      map.addLayer(layer, style.layers[i + 1]?.id);
  }
}
