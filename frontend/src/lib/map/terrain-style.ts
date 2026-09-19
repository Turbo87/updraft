import type { Map } from 'maplibre-gl';
import type { DerivedWindInstruments } from '$lib/protocol/generated/DerivedWindInstruments';
import type { HillshadeDirection } from '$lib/protocol/generated/HillshadeDirection';
import type { SolarPositionInstruments } from '$lib/protocol/generated/SolarPositionInstruments';

import { convertFileSrc } from '@tauri-apps/api/core';

type HillshadeLighting = {
  'hillshade-illumination-anchor': 'map' | 'viewport';
  'hillshade-illumination-direction': number;
};

type HillshadeInputs = {
  wind?: DerivedWindInstruments | null;
  solarPosition?: SolarPositionInstruments | null;
};

export function hillshadeLighting(
  direction: HillshadeDirection,
  { wind, solarPosition }: HillshadeInputs = {},
): HillshadeLighting {
  switch (direction) {
    case 'fixed':
      return fixedLighting();
    case 'wind':
      return wind ? mapLighting(wind.directionDegrees) : fixedLighting();
    case 'sun':
      return solarPosition ? mapLighting(solarPosition.azimuthDegrees) : fixedLighting();
  }
  direction satisfies never;
}

function fixedLighting(): HillshadeLighting {
  return {
    'hillshade-illumination-anchor': 'viewport',
    'hillshade-illumination-direction': 335,
  };
}

function mapLighting(direction: number): HillshadeLighting {
  return {
    'hillshade-illumination-anchor': 'map',
    'hillshade-illumination-direction': direction,
  };
}

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
