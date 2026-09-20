import type { DerivedWindInstruments } from '$lib/protocol/generated/DerivedWindInstruments';
import type { HillshadeDirection } from '$lib/protocol/generated/HillshadeDirection';
import type { SolarPositionInstruments } from '$lib/protocol/generated/SolarPositionInstruments';

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
