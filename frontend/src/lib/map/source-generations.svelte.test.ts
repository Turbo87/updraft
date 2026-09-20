import type { Map as MapLibreMap } from 'maplibre-gl';

import { addProtocol, removeProtocol } from 'maplibre-gl';
import { afterEach, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';

import { MapState } from '$lib/map-state.svelte';
import { TrafficStore } from '$lib/stores/traffic.svelte';
import Map from './Map.svelte';

vi.mock('@tauri-apps/api/core', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tauri-apps/api/core')>()),
  convertFileSrc: (path: string) => `generation://localhost/${encodeURIComponent(path)}`,
}));
vi.mock('./style/positron.json', () => ({
  default: {
    version: 8,
    sources: {},
    layers: [
      { id: 'water', type: 'background' },
      { id: 'waterway', type: 'circle', source: 'openmaptiles', 'source-layer': 'test' },
    ],
  },
}));

afterEach(() => removeProtocol('generation'));

function cameraAndLayers(map: MapLibreMap) {
  return {
    center: map.getCenter().toArray(),
    zoom: map.getZoom(),
    bearing: map.getBearing(),
    pitch: map.getPitch(),
    layers: map.getStyle().layers.map((layer) => layer.id),
  };
}

function pointTile(id: number): ArrayBuffer {
  let bytes = Uint8Array.fromHex('1a180a0474657374120b08001801220509802080202880207802');
  bytes[11] = id;
  return bytes.buffer;
}

it.each(['basemap', 'terrain-metadata', 'terrain-tiles', 'both'] as const)(
  'replaces %s generations and cancels pending requests without moving the map',
  async (resource) => {
    let updateBasemap = resource === 'basemap' || resource === 'both';
    let updateTerrain = resource !== 'basemap';
    let pending: { signal: AbortSignal; kind: string }[] = [];
    let terrainTiles = new Set<number>();
    let completeOldTiles: (() => void)[] = [];
    let images = await Promise.all(
      [256, 512].map(async (size) => {
        let canvas = document.createElement('canvas');
        canvas.width = canvas.height = size;
        let context = canvas.getContext('2d')!;
        context.fillStyle = 'rgb(128, 0, 0)';
        context.fillRect(0, 0, size, size);
        return await (await fetch(canvas.toDataURL('image/webp', 1))).arrayBuffer();
      }),
    );
    addProtocol('generation', async ({ url }, controller) => {
      let [, kind, version, path] = url.match(
        /^generation:\/\/localhost\/(basemap|terrain)\/(\d+)\/(.*)$/,
      )!;
      let generation = Number(version);
      let metadata = path === 'metadata.json';
      if (
        generation === 1 &&
        ((updateBasemap && kind === 'basemap') ||
          (kind === 'terrain' && (resource === 'terrain-metadata' || !metadata)))
      ) {
        pending.push({ signal: controller.signal, kind });
        if (kind === 'basemap') {
          return await new Promise<{ data: ArrayBuffer }>((resolve) => {
            completeOldTiles.push(() => resolve({ data: pointTile(7) }));
          });
        }
        return await new Promise<never>((_, reject) => {
          controller.signal.addEventListener('abort', () =>
            reject(new DOMException('Request aborted', 'AbortError')),
          );
        });
      }
      if (kind === 'basemap') {
        return {
          data: generation === 3 ? new ArrayBuffer(0) : pointTile(generation === 0 ? 7 : 9),
        };
      }
      if (metadata) {
        return {
          data: {
            tilejson: '3.0.0',
            encoding: 'terrarium',
            tileSize: generation === 0 ? 256 : 512,
            minzoom: 6,
            maxzoom: 6,
            attribution: `Terrain generation ${generation}`,
            tiles: [`updraft://localhost/terrain/${generation}/{z}/{x}/{y}.webp`],
          },
        };
      }
      terrainTiles.add(generation);
      return { data: images[generation === 0 ? 0 : 1].slice(0) };
    });
    let mapState = new MapState();
    mapState.followMode = false;
    mapState.center = { lat: 0, lng: 0 };
    mapState.zoom = 6;
    mapState.bearing = 12;
    mapState.pitch = 20;
    let component = await render(Map, {
      mapState,
      traffic: new TrafficStore(),
      airspace: { generation: 0, sources: [] },
      hillshadeDirection: 'fixed',
      instruments: {
        gps: null,
        pressureAltitude: null,
        trueAirspeed: null,
        terrainElevation: null,
        altitudeAgl: null,
        solarPosition: null,
        derived: null,
      },
      units: { altitude: 'm', distance: 'km', speed: 'km/h', verticalSpeed: 'm/s' },
    });
    await vi.waitFor(() => expect(mapState.map).toBeDefined());
    let map = mapState.map!;
    let errors: string[] = [];
    map.on('error', ({ error }) => errors.push(error.message));
    await vi.waitFor(() => expect(map.getLayer('traffic-fixed')).toBeDefined());
    function features() {
      return [
        ...new Set(
          map.querySourceFeatures('openmaptiles', { sourceLayer: 'test' }).map((f) => f.id),
        ),
      ];
    }
    function terrainMetadata() {
      let source = map.getSource('terrain')!;
      return {
        attribution: source.attribution,
        tileSize: 'tileSize' in source ? source.tileSize : null,
      };
    }
    await vi.waitFor(() => expect(features()).toEqual([7]));
    await vi.waitFor(() =>
      expect(terrainMetadata()).toEqual({ attribution: 'Terrain generation 0', tileSize: 256 }),
    );
    await vi.waitFor(() => expect([...terrainTiles]).toEqual([0]));
    let initial = cameraAndLayers(map);
    let trafficSource = map.getSource('traffic');
    function generations(generation: number) {
      return {
        ...(updateBasemap && { basemapGeneration: generation }),
        ...(updateTerrain && { terrainGeneration: generation }),
      };
    }
    await component.rerender(generations(1));
    await vi.waitFor(() =>
      expect(new Set(pending.map(({ kind }) => kind)).size).toBe(resource === 'both' ? 2 : 1),
    );
    if (updateBasemap) expect(features()).toEqual([]);
    if (updateTerrain) expect(terrainMetadata().attribution).not.toBe('Terrain generation 0');
    await component.rerender(generations(2));
    await vi.waitFor(() => expect(pending.every(({ signal }) => signal.aborted)).toBe(true));
    if (updateBasemap) {
      await vi.waitFor(() => expect(features()).toEqual([9]));
    }
    if (updateTerrain) {
      await vi.waitFor(() =>
        expect(terrainMetadata()).toEqual({ attribution: 'Terrain generation 2', tileSize: 512 }),
      );
      await vi.waitFor(() => expect([...terrainTiles]).toEqual([0, 2]));
    }
    for (let complete of completeOldTiles) complete();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    if (updateBasemap) expect(features()).toEqual([9]);
    await component.rerender(generations(3));
    if (updateBasemap) {
      await vi.waitFor(() => expect(features()).toEqual([]));
    }
    if (updateTerrain) {
      await vi.waitFor(() =>
        expect(terrainMetadata()).toEqual({ attribution: 'Terrain generation 3', tileSize: 512 }),
      );
      await vi.waitFor(() => expect([...terrainTiles]).toEqual([0, 2, 3]));
    }
    expect(cameraAndLayers(map)).toEqual(initial);
    expect(map.getSource('traffic')).toBe(trafficSource);
    expect(errors).toEqual([]);
  },
);
