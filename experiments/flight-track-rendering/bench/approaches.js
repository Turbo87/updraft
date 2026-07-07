// The rendering approaches under test.
//
// Each approach implements:
//   init(map, track, mode, initCount)  – add sources/layers for fixes [0, initCount)
//   append(map, track, i, mode)        – extend the track by fix i (called once per new fix)
//   recolor(map, track, mode)          – switch between 'alt' and 'vario' coloring
//   destroy(map)                       – remove everything again
//
// All approaches render a ~2px line where each segment between two fixes is
// colored by the altitude (or vario) at its newer endpoint.

import { rgbString, maplibreRampExpression, ALT_STOPS, VARIO_STOPS } from './igc.js';

const LINE_WIDTH = 2;

function rampExpression(mode) {
	return mode === 'alt'
		? maplibreRampExpression('a', ALT_STOPS)
		: maplibreRampExpression('v', VARIO_STOPS);
}

function segmentCoords(track, i) {
	const c = track.coords;
	return [
		[c[(i - 1) * 2], c[(i - 1) * 2 + 1]],
		[c[i * 2], c[i * 2 + 1]],
	];
}

// ---------------------------------------------------------------------------
// A. One GeoJSON feature per segment, pre-computed color string property.
//    Appends re-send the whole FeatureCollection via setData().
// ---------------------------------------------------------------------------
export const segmentsPrecolored = {
	name: 'geojson-segments-precolored',

	init(map, track, mode, initCount) {
		const colors = mode === 'alt' ? track.altColors : track.varioColors;
		this.features = [];
		for (let i = 1; i < initCount; i++) {
			this.features.push({
				type: 'Feature',
				geometry: { type: 'LineString', coordinates: segmentCoords(track, i) },
				properties: { c: rgbString(colors[i]) },
			});
		}
		this.data = { type: 'FeatureCollection', features: this.features };
		map.addSource('track', { type: 'geojson', data: this.data });
		map.addLayer({
			id: 'track',
			type: 'line',
			source: 'track',
			paint: { 'line-color': ['get', 'c'], 'line-width': LINE_WIDTH },
		});
	},

	append(map, track, i, mode) {
		const colors = mode === 'alt' ? track.altColors : track.varioColors;
		this.features.push({
			type: 'Feature',
			geometry: { type: 'LineString', coordinates: segmentCoords(track, i) },
			properties: { c: rgbString(colors[i]) },
		});
		map.getSource('track').setData(this.data);
	},

	recolor(map, track, mode) {
		const colors = mode === 'alt' ? track.altColors : track.varioColors;
		for (let k = 0; k < this.features.length; k++) {
			// feature k covers segment ending at fix k+1
			this.features[k].properties.c = rgbString(colors[k + 1]);
		}
		map.getSource('track').setData(this.data);
	},

	destroy(map) {
		map.removeLayer('track');
		map.removeSource('track');
		this.features = this.data = null;
	},
};

// ---------------------------------------------------------------------------
// B. One GeoJSON feature per segment with raw alt/vario properties and a
//    data-driven `interpolate` expression. Recoloring is setPaintProperty().
// ---------------------------------------------------------------------------
export const segmentsDataDriven = {
	name: 'geojson-segments-data-driven',

	makeFeature(track, i) {
		return {
			type: 'Feature',
			geometry: { type: 'LineString', coordinates: segmentCoords(track, i) },
			properties: {
				a: track.fixes[i].alt,
				v: Math.round(track.vario[i] * 100) / 100,
			},
		};
	},

	init(map, track, mode, initCount) {
		this.features = [];
		for (let i = 1; i < initCount; i++) this.features.push(this.makeFeature(track, i));
		this.data = { type: 'FeatureCollection', features: this.features };
		map.addSource('track', { type: 'geojson', data: this.data });
		map.addLayer({
			id: 'track',
			type: 'line',
			source: 'track',
			paint: { 'line-color': rampExpression(mode), 'line-width': LINE_WIDTH },
		});
	},

	append(map, track, i) {
		this.features.push(this.makeFeature(track, i));
		map.getSource('track').setData(this.data);
	},

	recolor(map, track, mode) {
		map.setPaintProperty('track', 'line-color', rampExpression(mode));
	},

	destroy(map) {
		map.removeLayer('track');
		map.removeSource('track');
		this.features = this.data = null;
	},
};

// ---------------------------------------------------------------------------
// B2. Same as B, but appends use the incremental GeoJSONSource.updateData()
//     diff API instead of re-sending the whole FeatureCollection.
// ---------------------------------------------------------------------------
export const segmentsUpdateData = {
	name: 'geojson-segments-updatedata',
	makeFeature: segmentsDataDriven.makeFeature,

	init(map, track, mode, initCount) {
		segmentsDataDriven.init.call(this, map, track, mode, initCount);
		// updateData() refuses to work unless every feature has a unique id
		for (let k = 0; k < this.features.length; k++) this.features[k].id = k + 1;
		map.getSource('track').setData(this.data);
	},

	append(map, track, i) {
		const feature = segmentsDataDriven.makeFeature(track, i);
		feature.id = i;
		this.features.push(feature);
		map.getSource('track').updateData({ add: [feature] });
	},

	recolor(map, track, mode) {
		map.setPaintProperty('track', 'line-color', rampExpression(mode));
	},

	destroy(map) {
		segmentsDataDriven.destroy.call(this, map);
	},
};

// ---------------------------------------------------------------------------
// C. Single LineString with lineMetrics + line-gradient over line-progress.
//    The gradient expression is down-sampled to <= MAX_STOPS stops. Every
//    append shifts all line-progress values, so the gradient must be rebuilt.
// ---------------------------------------------------------------------------
export const singleLineGradient = {
	name: 'geojson-line-gradient',
	MAX_STOPS: 512,

	cumulativeDistances(track, count) {
		// equirectangular approximation is plenty for progress values
		const c = track.coords;
		const dist = new Float64Array(count);
		let total = 0;
		for (let i = 1; i < count; i++) {
			const dx = (c[i * 2] - c[(i - 1) * 2]) * Math.cos((c[i * 2 + 1] * Math.PI) / 180);
			const dy = c[i * 2 + 1] - c[(i - 1) * 2 + 1];
			total += Math.sqrt(dx * dx + dy * dy);
			dist[i] = total;
		}
		return dist;
	},

	gradientExpression(track, count, mode) {
		const colors = mode === 'alt' ? track.altColors : track.varioColors;
		const dist = this.cumulativeDistances(track, count);
		const total = dist[count - 1] || 1;
		const step = Math.max(1, Math.ceil(count / this.MAX_STOPS));
		const expr = ['interpolate', ['linear'], ['line-progress']];
		let last = -1;
		for (let i = 0; i < count; i += step) {
			const p = dist[i] / total;
			if (p <= last) continue;
			last = p;
			expr.push(p, rgbString(colors[i]));
		}
		return expr;
	},

	init(map, track, mode, initCount) {
		this.count = initCount;
		this.coordinates = [];
		for (let i = 0; i < initCount; i++) {
			this.coordinates.push([track.coords[i * 2], track.coords[i * 2 + 1]]);
		}
		this.data = {
			type: 'Feature',
			geometry: { type: 'LineString', coordinates: this.coordinates },
			properties: {},
		};
		map.addSource('track', { type: 'geojson', data: this.data, lineMetrics: true });
		map.addLayer({
			id: 'track',
			type: 'line',
			source: 'track',
			paint: {
				'line-gradient': this.gradientExpression(track, initCount, mode),
				'line-width': LINE_WIDTH,
			},
		});
	},

	append(map, track, i, mode) {
		this.coordinates.push([track.coords[i * 2], track.coords[i * 2 + 1]]);
		this.count = i + 1;
		map.getSource('track').setData(this.data);
		map.setPaintProperty('track', 'line-gradient', this.gradientExpression(track, this.count, mode));
	},

	recolor(map, track, mode) {
		map.setPaintProperty('track', 'line-gradient', this.gradientExpression(track, this.count, mode));
	},

	destroy(map) {
		map.removeLayer('track');
		map.removeSource('track');
		this.coordinates = this.data = null;
	},
};

// ---------------------------------------------------------------------------
// D. Two sources: a big "static" one that is only re-sent when the live
//    buffer overflows, and a tiny "live" one that receives the 1 Hz appends.
// ---------------------------------------------------------------------------
export const chunkedStaticLive = {
	name: 'geojson-chunked-static-live',
	LIVE_LIMIT: 60,

	init(map, track, mode, initCount) {
		this.staticFeatures = [];
		for (let i = 1; i < initCount; i++) {
			this.staticFeatures.push(segmentsDataDriven.makeFeature(track, i));
		}
		this.liveFeatures = [];
		this.staticData = { type: 'FeatureCollection', features: this.staticFeatures };
		this.liveData = { type: 'FeatureCollection', features: this.liveFeatures };
		map.addSource('track-static', { type: 'geojson', data: this.staticData });
		map.addSource('track-live', { type: 'geojson', data: this.liveData });
		for (const part of ['static', 'live']) {
			map.addLayer({
				id: `track-${part}`,
				type: 'line',
				source: `track-${part}`,
				paint: { 'line-color': rampExpression(mode), 'line-width': LINE_WIDTH },
			});
		}
	},

	append(map, track, i) {
		this.liveFeatures.push(segmentsDataDriven.makeFeature(track, i));
		if (this.liveFeatures.length >= this.LIVE_LIMIT) {
			// merge the live buffer into the static source
			this.staticFeatures.push(...this.liveFeatures);
			this.liveFeatures.length = 0;
			map.getSource('track-static').setData(this.staticData);
		}
		map.getSource('track-live').setData(this.liveData);
	},

	recolor(map, track, mode) {
		map.setPaintProperty('track-static', 'line-color', rampExpression(mode));
		map.setPaintProperty('track-live', 'line-color', rampExpression(mode));
	},

	destroy(map) {
		for (const part of ['static', 'live']) {
			map.removeLayer(`track-${part}`);
			map.removeSource(`track-${part}`);
		}
		this.staticFeatures = this.liveFeatures = this.staticData = this.liveData = null;
	},
};

// ---------------------------------------------------------------------------
// E. Per-segment features colored through feature-state.
// ---------------------------------------------------------------------------
export const featureState = {
	name: 'geojson-feature-state',

	init(map, track, mode, initCount) {
		const colors = mode === 'alt' ? track.altColors : track.varioColors;
		this.features = [];
		for (let i = 1; i < initCount; i++) {
			this.features.push({
				type: 'Feature',
				id: i,
				geometry: { type: 'LineString', coordinates: segmentCoords(track, i) },
				properties: {},
			});
		}
		this.data = { type: 'FeatureCollection', features: this.features };
		this.count = initCount;
		map.addSource('track', { type: 'geojson', data: this.data });
		map.addLayer({
			id: 'track',
			type: 'line',
			source: 'track',
			paint: {
				'line-color': ['to-color', ['coalesce', ['feature-state', 'c'], '#888888']],
				'line-width': LINE_WIDTH,
			},
		});
		for (let i = 1; i < initCount; i++) {
			map.setFeatureState({ source: 'track', id: i }, { c: rgbString(colors[i]) });
		}
	},

	append(map, track, i, mode) {
		const colors = mode === 'alt' ? track.altColors : track.varioColors;
		this.features.push({
			type: 'Feature',
			id: i,
			geometry: { type: 'LineString', coordinates: segmentCoords(track, i) },
			properties: {},
		});
		this.count = i + 1;
		map.getSource('track').setData(this.data);
		map.setFeatureState({ source: 'track', id: i }, { c: rgbString(colors[i]) });
	},

	recolor(map, track, mode) {
		const colors = mode === 'alt' ? track.altColors : track.varioColors;
		for (let i = 1; i < this.count; i++) {
			map.setFeatureState({ source: 'track', id: i }, { c: rgbString(colors[i]) });
		}
	},

	destroy(map) {
		map.removeLayer('track');
		map.removeSource('track');
		this.features = this.data = null;
	},
};

// ---------------------------------------------------------------------------
// F. Custom WebGL layer: one triangle-strip quad per segment, per-vertex
//    colors for both modes kept in separate GPU buffers (recolor = rebind),
//    appends via gl.bufferSubData. Extrusion to pixel width happens in the
//    vertex shader, so no re-tessellation on zoom.
// ---------------------------------------------------------------------------
export const customWebGL = {
	name: 'custom-webgl-layer',

	init(map, track, mode, initCount) {
		const self = this;
		this.map = map;
		this.mode = mode;
		this.track = track;

		// precompute mercator positions relative to the first fix (keeps
		// Float32 precision; the anchor is added back via the matrix)
		const n = track.n;
		const merc = new Float64Array(n * 2);
		for (let i = 0; i < n; i++) {
			const lon = track.coords[i * 2];
			const lat = track.coords[i * 2 + 1];
			merc[i * 2] = (180 + lon) / 360;
			merc[i * 2 + 1] =
				(180 - (180 / Math.PI) * Math.log(Math.tan(Math.PI / 4 + (lat * Math.PI) / 360))) / 360;
		}
		this.anchor = [merc[0], merc[1]];
		this.merc = merc;

		this.layer = {
			id: 'track-custom',
			type: 'custom',
			renderingMode: '2d',

			onAdd(m, gl) {
				const vs = `#version 300 es
					uniform mat4 u_matrix;
					uniform vec2 u_viewport;
					uniform float u_half_width;
					in vec2 a_pos;
					in vec2 a_norm;
					in vec4 a_color;
					out vec4 v_color;
					void main() {
						vec4 p = u_matrix * vec4(a_pos, 0.0, 1.0);
						vec4 q = u_matrix * vec4(a_pos + a_norm * 1e-5, 0.0, 1.0);
						vec2 screen_dir = normalize((q.xy / q.w - p.xy / p.w) * u_viewport);
						p.xy += screen_dir * u_half_width * 2.0 / u_viewport * p.w;
						gl_Position = p;
						v_color = a_color;
					}`;
				const fs = `#version 300 es
					precision mediump float;
					in vec4 v_color;
					out vec4 fragColor;
					void main() { fragColor = v_color; }`;

				const compile = (type, src) => {
					const s = gl.createShader(type);
					gl.shaderSource(s, src);
					gl.compileShader(s);
					if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
						throw new Error(gl.getShaderInfoLog(s));
					}
					return s;
				};
				const prog = gl.createProgram();
				gl.attachShader(prog, compile(gl.VERTEX_SHADER, vs));
				gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, fs));
				gl.linkProgram(prog);
				if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
					throw new Error(gl.getProgramInfoLog(prog));
				}
				this.program = prog;
				this.loc = {
					matrix: gl.getUniformLocation(prog, 'u_matrix'),
					viewport: gl.getUniformLocation(prog, 'u_viewport'),
					halfWidth: gl.getUniformLocation(prog, 'u_half_width'),
					pos: gl.getAttribLocation(prog, 'a_pos'),
					norm: gl.getAttribLocation(prog, 'a_norm'),
					color: gl.getAttribLocation(prog, 'a_color'),
				};

				// preallocate for the full flight: 4 verts / 6 indices per segment
				const capSegments = n; // n-1 segments needed, 1 spare
				this.posBuf = gl.createBuffer();
				gl.bindBuffer(gl.ARRAY_BUFFER, this.posBuf);
				gl.bufferData(gl.ARRAY_BUFFER, capSegments * 4 * 2 * 4, gl.DYNAMIC_DRAW);
				this.normBuf = gl.createBuffer();
				gl.bindBuffer(gl.ARRAY_BUFFER, this.normBuf);
				gl.bufferData(gl.ARRAY_BUFFER, capSegments * 4 * 2 * 4, gl.DYNAMIC_DRAW);
				this.altColorBuf = gl.createBuffer();
				gl.bindBuffer(gl.ARRAY_BUFFER, this.altColorBuf);
				gl.bufferData(gl.ARRAY_BUFFER, capSegments * 4 * 4, gl.DYNAMIC_DRAW);
				this.varioColorBuf = gl.createBuffer();
				gl.bindBuffer(gl.ARRAY_BUFFER, this.varioColorBuf);
				gl.bufferData(gl.ARRAY_BUFFER, capSegments * 4 * 4, gl.DYNAMIC_DRAW);
				this.idxBuf = gl.createBuffer();
				gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.idxBuf);
				gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, capSegments * 6 * 4, gl.DYNAMIC_DRAW);

				this.segmentCount = 0;
				this.gl = gl;

				// upload the initial segments in one go
				const segs = initCount - 1;
				const pos = new Float32Array(segs * 4 * 2);
				const norm = new Float32Array(segs * 4 * 2);
				const altCol = new Uint8Array(segs * 4 * 4);
				const varCol = new Uint8Array(segs * 4 * 4);
				const idx = new Uint32Array(segs * 6);
				for (let i = 1; i < initCount; i++) {
					self.fillSegment(i, pos, norm, altCol, varCol, idx, i - 1);
				}
				gl.bindBuffer(gl.ARRAY_BUFFER, this.posBuf);
				gl.bufferSubData(gl.ARRAY_BUFFER, 0, pos);
				gl.bindBuffer(gl.ARRAY_BUFFER, this.normBuf);
				gl.bufferSubData(gl.ARRAY_BUFFER, 0, norm);
				gl.bindBuffer(gl.ARRAY_BUFFER, this.altColorBuf);
				gl.bufferSubData(gl.ARRAY_BUFFER, 0, altCol);
				gl.bindBuffer(gl.ARRAY_BUFFER, this.varioColorBuf);
				gl.bufferSubData(gl.ARRAY_BUFFER, 0, varCol);
				gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.idxBuf);
				gl.bufferSubData(gl.ELEMENT_ARRAY_BUFFER, 0, idx);
				this.segmentCount = segs;
			},

			render(gl, args) {
				const matrix = args.defaultProjectionData
					? args.defaultProjectionData.mainMatrix
					: args; // pre-v5 fallback: args IS the matrix
				// bake the anchor translation into the matrix (in doubles)
				const m = Array.from(matrix);
				const [ax, ay] = self.anchor;
				m[12] += m[0] * ax + m[4] * ay;
				m[13] += m[1] * ax + m[5] * ay;
				m[14] += m[2] * ax + m[6] * ay;
				m[15] += m[3] * ax + m[7] * ay;

				gl.useProgram(this.program);
				gl.uniformMatrix4fv(this.loc.matrix, false, new Float32Array(m));
				gl.uniform2f(this.loc.viewport, gl.canvas.width, gl.canvas.height);
				gl.uniform1f(this.loc.halfWidth, (LINE_WIDTH / 2) * devicePixelRatio);

				gl.bindBuffer(gl.ARRAY_BUFFER, this.posBuf);
				gl.enableVertexAttribArray(this.loc.pos);
				gl.vertexAttribPointer(this.loc.pos, 2, gl.FLOAT, false, 0, 0);
				gl.bindBuffer(gl.ARRAY_BUFFER, this.normBuf);
				gl.enableVertexAttribArray(this.loc.norm);
				gl.vertexAttribPointer(this.loc.norm, 2, gl.FLOAT, false, 0, 0);
				gl.bindBuffer(
					gl.ARRAY_BUFFER,
					self.mode === 'alt' ? this.altColorBuf : this.varioColorBuf,
				);
				gl.enableVertexAttribArray(this.loc.color);
				gl.vertexAttribPointer(this.loc.color, 4, gl.UNSIGNED_BYTE, true, 0, 0);
				gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.idxBuf);

				gl.enable(gl.BLEND);
				gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
				gl.drawElements(gl.TRIANGLES, this.segmentCount * 6, gl.UNSIGNED_INT, 0);
			},

			onRemove(m, gl) {
				for (const b of [this.posBuf, this.normBuf, this.altColorBuf, this.varioColorBuf, this.idxBuf]) {
					gl.deleteBuffer(b);
				}
				gl.deleteProgram(this.program);
			},
		};

		map.addLayer(this.layer);
	},

	// write one segment quad into the given arrays at slot `slot`
	fillSegment(i, pos, norm, altCol, varCol, idx, slot) {
		const { merc, anchor, track } = this;
		const x0 = merc[(i - 1) * 2] - anchor[0];
		const y0 = merc[(i - 1) * 2 + 1] - anchor[1];
		const x1 = merc[i * 2] - anchor[0];
		const y1 = merc[i * 2 + 1] - anchor[1];
		let nx = -(y1 - y0);
		let ny = x1 - x0;
		const len = Math.hypot(nx, ny) || 1;
		nx /= len;
		ny /= len;

		const pBase = slot * 8;
		pos[pBase] = x0; pos[pBase + 1] = y0;
		pos[pBase + 2] = x0; pos[pBase + 3] = y0;
		pos[pBase + 4] = x1; pos[pBase + 5] = y1;
		pos[pBase + 6] = x1; pos[pBase + 7] = y1;
		norm[pBase] = nx; norm[pBase + 1] = ny;
		norm[pBase + 2] = -nx; norm[pBase + 3] = -ny;
		norm[pBase + 4] = nx; norm[pBase + 5] = ny;
		norm[pBase + 6] = -nx; norm[pBase + 7] = -ny;

		const [ar, ag, ab] = track.altColors[i];
		const [vr, vg, vb] = track.varioColors[i];
		const cBase = slot * 16;
		for (let v = 0; v < 4; v++) {
			altCol[cBase + v * 4] = ar;
			altCol[cBase + v * 4 + 1] = ag;
			altCol[cBase + v * 4 + 2] = ab;
			altCol[cBase + v * 4 + 3] = 255;
			varCol[cBase + v * 4] = vr;
			varCol[cBase + v * 4 + 1] = vg;
			varCol[cBase + v * 4 + 2] = vb;
			varCol[cBase + v * 4 + 3] = 255;
		}

		const iBase = slot * 6;
		const vBase = slot * 4;
		idx[iBase] = vBase;
		idx[iBase + 1] = vBase + 1;
		idx[iBase + 2] = vBase + 2;
		idx[iBase + 3] = vBase + 1;
		idx[iBase + 4] = vBase + 3;
		idx[iBase + 5] = vBase + 2;
	},

	append(map, track, i) {
		const layer = this.layer;
		const gl = layer.gl;
		const pos = new Float32Array(8);
		const norm = new Float32Array(8);
		const altCol = new Uint8Array(16);
		const varCol = new Uint8Array(16);
		const idx = new Uint32Array(6);
		const slot = layer.segmentCount;
		this.fillSegment(i, pos, norm, altCol, varCol, idx, 0);
		// fix up index values for the real slot
		for (let k = 0; k < 6; k++) idx[k] += slot * 4 - 0 * 4;
		gl.bindBuffer(gl.ARRAY_BUFFER, layer.posBuf);
		gl.bufferSubData(gl.ARRAY_BUFFER, slot * 8 * 4, pos);
		gl.bindBuffer(gl.ARRAY_BUFFER, layer.normBuf);
		gl.bufferSubData(gl.ARRAY_BUFFER, slot * 8 * 4, norm);
		gl.bindBuffer(gl.ARRAY_BUFFER, layer.altColorBuf);
		gl.bufferSubData(gl.ARRAY_BUFFER, slot * 16, altCol);
		gl.bindBuffer(gl.ARRAY_BUFFER, layer.varioColorBuf);
		gl.bufferSubData(gl.ARRAY_BUFFER, slot * 16, varCol);
		gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, layer.idxBuf);
		gl.bufferSubData(gl.ELEMENT_ARRAY_BUFFER, slot * 6 * 4, idx);
		layer.segmentCount++;
		map.triggerRepaint();
	},

	recolor(map, track, mode) {
		this.mode = mode;
		map.triggerRepaint();
	},

	destroy(map) {
		map.removeLayer('track-custom');
		this.layer = this.merc = this.track = null;
	},
};

// ---------------------------------------------------------------------------
// G. deck.gl LineLayer through MapboxOverlay (interleaved with maplibre).
//    One instanced line per segment; deck re-uploads attributes on append.
// ---------------------------------------------------------------------------
export const deckglLineLayer = {
	name: 'deckgl-line-layer',

	makeLayer(mode) {
		return new deck.LineLayer({
			id: 'track-deck',
			data: this.segments,
			getSourcePosition: (d) => d.s,
			getTargetPosition: (d) => d.t,
			getColor: mode === 'alt' ? (d) => d.ac : (d) => d.vc,
			getWidth: LINE_WIDTH,
			widthUnits: 'pixels',
			updateTriggers: { getColor: mode },
		});
	},

	init(map, track, mode, initCount) {
		this.segments = [];
		for (let i = 1; i < initCount; i++) {
			this.segments.push({
				s: segmentCoords(track, i)[0],
				t: segmentCoords(track, i)[1],
				ac: track.altColors[i],
				vc: track.varioColors[i],
			});
		}
		this.overlay = new deck.MapboxOverlay({
			interleaved: true,
			layers: [this.makeLayer(mode)],
		});
		map.addControl(this.overlay);
	},

	append(map, track, i, mode) {
		this.segments = this.segments.concat([
			{
				s: segmentCoords(track, i)[0],
				t: segmentCoords(track, i)[1],
				ac: track.altColors[i],
				vc: track.varioColors[i],
			},
		]);
		this.overlay.setProps({ layers: [this.makeLayer(mode)] });
	},

	recolor(map, track, mode) {
		this.overlay.setProps({ layers: [this.makeLayer(mode)] });
	},

	destroy(map) {
		map.removeControl(this.overlay);
		this.overlay = this.segments = null;
	},
};

export const APPROACHES = [
	segmentsPrecolored,
	segmentsDataDriven,
	segmentsUpdateData,
	singleLineGradient,
	chunkedStaticLive,
	featureState,
	customWebGL,
	deckglLineLayer,
];
