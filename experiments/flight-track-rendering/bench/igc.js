// Minimal IGC parsing + derived data for the rendering benchmark.
//
// Produces a flat fix array plus per-fix vario and per-fix colors so every
// rendering approach starts from identical inputs and none of them pays
// different preprocessing costs inside the measured sections.

export function parseIGC(text) {
	const fixes = [];
	let prevTime = -1;
	let dayOffset = 0;
	for (const line of text.split('\n')) {
		if (line.charCodeAt(0) !== 66 /* 'B' */) continue;
		// B HHMMSS DDMMmmm[NS] DDDMMmmm[EW] A PPPPP GGGGG ...
		const h = +line.slice(1, 3);
		const m = +line.slice(3, 5);
		const s = +line.slice(5, 7);
		let time = h * 3600 + m * 60 + s;
		if (time + dayOffset < prevTime) dayOffset += 86400;
		time += dayOffset;
		prevTime = time;

		const latDeg = +line.slice(7, 9);
		const latMin = +line.slice(9, 14) / 1000;
		let lat = latDeg + latMin / 60;
		if (line[14] === 'S') lat = -lat;

		const lonDeg = +line.slice(15, 18);
		const lonMin = +line.slice(18, 23) / 1000;
		let lon = lonDeg + lonMin / 60;
		if (line[23] === 'W') lon = -lon;

		if (line[24] !== 'A') continue; // only valid 3D fixes
		const pressAlt = +line.slice(25, 30);
		const gpsAlt = +line.slice(30, 35);

		fixes.push({ time, lat, lon, alt: pressAlt || gpsAlt });
	}
	return fixes;
}

// Centered moving-average vario over ±window seconds, in m/s.
export function computeVario(fixes, windowSec = 2) {
	const n = fixes.length;
	const vario = new Float32Array(n);
	for (let i = 0; i < n; i++) {
		const lo = Math.max(0, i - windowSec);
		const hi = Math.min(n - 1, i + windowSec);
		const dt = fixes[hi].time - fixes[lo].time;
		vario[i] = dt > 0 ? (fixes[hi].alt - fixes[lo].alt) / dt : 0;
	}
	return vario;
}

// --- color scales -----------------------------------------------------------

function lerp(a, b, t) {
	return a + (b - a) * t;
}

function rampColor(stops, value) {
	if (value <= stops[0][0]) return stops[0].slice(1);
	for (let i = 1; i < stops.length; i++) {
		if (value <= stops[i][0]) {
			const t = (value - stops[i - 1][0]) / (stops[i][0] - stops[i - 1][0]);
			return [
				Math.round(lerp(stops[i - 1][1], stops[i][1], t)),
				Math.round(lerp(stops[i - 1][2], stops[i][2], t)),
				Math.round(lerp(stops[i - 1][3], stops[i][3], t)),
			];
		}
	}
	return stops[stops.length - 1].slice(1);
}

// altitude: 0 m .. 3000 m, dark blue -> cyan -> green -> yellow -> red
export const ALT_STOPS = [
	[0, 26, 42, 122],
	[750, 0, 169, 206],
	[1500, 22, 176, 91],
	[2250, 242, 216, 32],
	[3000, 220, 38, 38],
];

// vario: -5 .. +5 m/s, blue -> grey -> red
export const VARIO_STOPS = [
	[-5, 30, 64, 175],
	[0, 156, 163, 175],
	[5, 185, 28, 28],
];

export function altColor(alt) {
	return rampColor(ALT_STOPS, alt);
}

export function varioColor(v) {
	return rampColor(VARIO_STOPS, v);
}

export function rgbString([r, g, b]) {
	return `rgb(${r},${g},${b})`;
}

// MapLibre `interpolate` expression matching the JS ramps above, so
// data-driven styling approaches produce identical colors.
export function maplibreRampExpression(property, stops) {
	const expr = ['interpolate', ['linear'], ['get', property]];
	for (const [v, r, g, b] of stops) expr.push(v, `rgb(${r},${g},${b})`);
	return expr;
}

// Precompute everything each approach might need.
export function prepareTrack(fixes) {
	const n = fixes.length;
	const vario = computeVario(fixes);
	const coords = new Float64Array(n * 2);
	const altColors = new Array(n);
	const varioColors = new Array(n);
	for (let i = 0; i < n; i++) {
		coords[i * 2] = fixes[i].lon;
		coords[i * 2 + 1] = fixes[i].lat;
		altColors[i] = altColor(fixes[i].alt);
		varioColors[i] = varioColor(vario[i]);
	}
	return { fixes, vario, coords, altColors, varioColors, n };
}
