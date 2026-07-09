// Mock stand-in for the core's `query_at` hit-testing. Results are derived
// deterministically from the coordinate so that a deep link / refresh always
// yields the same list — mirroring how the real core would answer for a point.

export interface Coord {
  lat: number;
  lng: number;
}

export type FeatureKind = 'Airspace' | 'Airfield' | 'Waypoint' | 'Traffic';

export interface Feature {
  id: string;
  kind: FeatureKind;
  name: string;
  summary: string;
  detail: Record<string, string>;
}

/** Parse a `@lat,lng` path segment into a coordinate. */
export function parseCoord(segment: string): Coord {
  let [lat, lng] = segment.replace(/^@/, '').split(',').map(Number);
  return { lat, lng };
}

/** Format a coordinate as a `@lat,lng` path segment (4 dp). */
export function formatCoord(c: Coord): string {
  return `@${c.lat.toFixed(4)},${c.lng.toFixed(4)}`;
}

// Small deterministic PRNG seeded from the coordinate string.
function seeded(key: string): () => number {
  let h = 2166136261;
  for (let i = 0; i < key.length; i++) {
    h ^= key.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return () => {
    h += 0x6d2b79f5;
    let t = h;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Everything at/near a coordinate, deterministic per point. */
export function queryAt(coord: Coord): Feature[] {
  let rnd = seeded(formatCoord(coord));
  let pick = <T>(arr: T[]): T => arr[Math.floor(rnd() * arr.length)];
  let features: Feature[] = [];

  for (let i = 0; i < Math.floor(rnd() * 3); i++) {
    let cls = pick(['C', 'D', 'E']);
    let floor = pick(['GND', '1000 ft MSL', 'FL65']);
    features.push({
      id: `airspace-${i}`,
      kind: 'Airspace',
      name: `${pick(['AACHEN', 'NÖRVENICH', 'LIÈGE'])} CTR ${cls}`,
      summary: `Class ${cls} · ${floor} – FL120`,
      detail: { Class: cls, Floor: floor, Ceiling: 'FL120', Type: 'Controlled airspace' },
    });
  }

  if (rnd() > 0.4) {
    let elev = 100 + Math.floor(rnd() * 200);
    features.push({
      id: 'airfield-0',
      kind: 'Airfield',
      name: pick(['Merzbrück', 'Dahlemer Binz', 'Aachen-Merzbrück']),
      summary: `Airfield · ${elev} m`,
      detail: {
        Elevation: `${elev} m`,
        Runway: pick(['07/25 grass', '10/28 asphalt', '13/31 grass']),
        Frequency: '122.850 MHz',
        Traffic: 'Gliders, GA',
      },
    });
  }

  for (let i = 0; i < 1 + Math.floor(rnd() * 2); i++) {
    features.push({
      id: `waypoint-${i}`,
      kind: 'Waypoint',
      name: pick(['Rurtalsperre', 'Hohes Venn', 'Eifel Ridge', 'Dreiländereck']),
      summary: pick(['Turnpoint', 'Landmark', 'Thermal source']),
      detail: {
        Type: pick(['Turnpoint', 'Landmark']),
        Elevation: `${30 + Math.floor(rnd() * 150)} m`,
      },
    });
  }

  if (rnd() > 0.5) {
    features.push({
      id: 'traffic-0',
      kind: 'Traffic',
      name: `Glider ${pick(['D-1234', 'D-8842', 'OY-XKP'])}`,
      summary: 'FLARM · climbing',
      detail: { Type: 'Glider', Source: 'FLARM', State: 'Circling' },
    });
  }

  if (features.length === 0) {
    features.push({
      id: 'waypoint-0',
      kind: 'Waypoint',
      name: 'Open field',
      summary: 'Nothing notable here',
      detail: { Note: 'No registered features near this point' },
    });
  }

  return features;
}

/** A single feature at a coordinate, by id. */
export function getFeature(coord: Coord, id: string): Feature | undefined {
  return queryAt(coord).find((f) => f.id === id);
}
