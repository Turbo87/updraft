// Benchmark harness. Loaded by index.html; driven from Playwright (or the
// dev tools console) via `window.runBenchmark(approachName)`.

import { parseIGC, prepareTrack } from './igc.js';
import { APPROACHES } from './approaches.js';

const APPEND_COUNT = 60; // serialized 1 Hz-style appends to measure
const ANIMATED_APPEND_COUNT = 20; // appends fired while the camera animates

const BLANK_STYLE = {
	version: 8,
	sources: {},
	layers: [{ id: 'bg', type: 'background', paint: { 'background-color': '#e8e4dc' } }],
};

let trackPromise;
function loadTrack() {
	trackPromise ??= fetch('../flight.igc')
		.then((r) => r.text())
		.then((text) => prepareTrack(parseIGC(text)));
	return trackPromise;
}

function onceIdle(map, timeoutMs = 20000) {
	return new Promise((resolve, reject) => {
		const t = setTimeout(() => reject(new Error('idle timeout')), timeoutMs);
		map.once('idle', () => {
			clearTimeout(t);
			resolve();
		});
	});
}

function stats(samples) {
	const sorted = [...samples].sort((a, b) => a - b);
	const sum = samples.reduce((a, b) => a + b, 0);
	return {
		avg: sum / samples.length,
		p50: sorted[Math.floor(sorted.length * 0.5)],
		p95: sorted[Math.floor(sorted.length * 0.95)],
		max: sorted[sorted.length - 1],
	};
}

function trackBounds(track) {
	let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
	for (let i = 0; i < track.n; i++) {
		const x = track.coords[i * 2];
		const y = track.coords[i * 2 + 1];
		if (x < minX) minX = x;
		if (x > maxX) maxX = x;
		if (y < minY) minY = y;
		if (y > maxY) maxY = y;
	}
	return [[minX, minY], [maxX, maxY]];
}

// Count rAF frames while a camera animation runs; returns fps.
function measureAnimationFps(map, easeOptions) {
	return new Promise((resolve) => {
		let frames = 0;
		let done = false;
		const start = performance.now();
		function tick() {
			frames++;
			if (!done) requestAnimationFrame(tick);
		}
		requestAnimationFrame(tick);
		map.once('moveend', () => {
			done = true;
			resolve((frames / (performance.now() - start)) * 1000);
		});
		map.easeTo({ ...easeOptions, duration: easeOptions.duration ?? 3000 });
	});
}

window.runBenchmark = async function runBenchmark(approachName, { mode = 'alt' } = {}) {
	const approach = APPROACHES.find((a) => a.name === approachName);
	if (!approach) throw new Error(`unknown approach: ${approachName}`);

	const track = await loadTrack();
	const bounds = trackBounds(track);

	const map = new maplibregl.Map({
		container: 'map',
		style: BLANK_STYLE,
		bounds,
		fitBoundsOptions: { padding: 40 },
		attributionControl: false,
		fadeDuration: 0,
	});
	map.on('error', (e) => console.error('map error:', e.error?.message ?? e));
	await new Promise((resolve) => map.once('load', resolve));

	const result = { approach: approachName, fixes: track.n };
	const initCount = track.n - APPEND_COUNT - ANIMATED_APPEND_COUNT;

	// --- initial load -------------------------------------------------------
	let t0 = performance.now();
	approach.init(map, track, mode, initCount);
	const initCallMs = performance.now() - t0;
	await onceIdle(map, 60000);
	result.initCallMs = initCallMs;
	result.initTotalMs = performance.now() - t0;

	// --- serialized appends (the 1 fix/second case) --------------------------
	const appendCall = [];
	const appendLatency = [];
	for (let k = 0; k < APPEND_COUNT; k++) {
		const i = initCount + k;
		t0 = performance.now();
		approach.append(map, track, i, mode);
		appendCall.push(performance.now() - t0);
		await onceIdle(map);
		appendLatency.push(performance.now() - t0);
	}
	result.appendCall = stats(appendCall);
	result.appendLatency = stats(appendLatency);

	// --- camera animation fps over the full track ----------------------------
	const [[minX, minY], [maxX, maxY]] = bounds;
	map.jumpTo({ center: [minX, minY], zoom: 11 });
	await onceIdle(map);
	result.panFps = await measureAnimationFps(map, {
		center: [maxX, maxY],
		zoom: 11,
	});
	result.zoomFps = await measureAnimationFps(map, {
		center: [(minX + maxX) / 2, (minY + maxY) / 2],
		zoom: 8,
		bearing: 30,
	});

	// --- appends while the camera animates (live-tracking case) --------------
	const fpsDuringAppends = await new Promise((resolve) => {
		let k = 0;
		const interval = setInterval(() => {
			if (k < ANIMATED_APPEND_COUNT) {
				approach.append(map, track, initCount + APPEND_COUNT + k, mode);
				k++;
			}
		}, 150);
		measureAnimationFps(map, {
			center: bounds[0],
			zoom: 10,
			bearing: 0,
			duration: 3000,
		}).then((fps) => {
			clearInterval(interval);
			resolve(fps);
		});
	});
	result.fpsDuringAppends = fpsDuringAppends;

	// --- recolor (alt <-> vario switch) --------------------------------------
	await onceIdle(map).catch(() => {});
	t0 = performance.now();
	approach.recolor(map, track, mode === 'alt' ? 'vario' : 'alt');
	const recolorCallMs = performance.now() - t0;
	await onceIdle(map, 60000);
	result.recolorCallMs = recolorCallMs;
	result.recolorTotalMs = performance.now() - t0;

	result.heapMB = performance.memory ? performance.memory.usedJSHeapSize / 1048576 : null;

	// zoom in on the final glide for the screenshot, then leave the map alive
	// so the driver can capture it; cleanupBenchmark() tears it down.
	map.jumpTo({ center: [track.coords[(track.n - 1) * 2], track.coords[(track.n - 1) * 2 + 1]], zoom: 10 });
	await onceIdle(map).catch(() => {});
	window.cleanupBenchmark = () => {
		approach.destroy(map);
		map.remove();
	};
	return result;
};

window.listApproaches = () => APPROACHES.map((a) => a.name);
