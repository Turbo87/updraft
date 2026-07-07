// Playwright driver: serves the bench directory, runs every approach in a
// fresh page, prints a markdown results table and writes results.json.

import http from 'node:http';
import { readFile, writeFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const root = fileURLToPath(new URL('.', import.meta.url));

const MIME = {
	'.html': 'text/html',
	'.js': 'text/javascript',
	'.css': 'text/css',
	'.igc': 'text/plain',
	'.json': 'application/json',
};

const server = http.createServer(async (req, res) => {
	try {
		const path = normalize(decodeURIComponent(new URL(req.url, 'http://x').pathname));
		const file = join(root, path === '/' ? 'bench/index.html' : path);
		const body = await readFile(file);
		res.writeHead(200, { 'content-type': MIME[extname(file)] ?? 'application/octet-stream' });
		res.end(body);
	} catch {
		res.writeHead(404);
		res.end('not found');
	}
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}/bench/index.html`;

const browser = await chromium.launch({
	headless: true,
	// use the environment's pre-installed Chromium if the pinned Playwright
	// revision is not present
	executablePath: process.env.CHROMIUM_PATH ?? '/opt/pw-browsers/chromium',
	args: ['--enable-precise-memory-info', '--disable-dev-shm-usage'],
});

const filter = process.argv[2];
const results = [];

const context = await browser.newContext({ viewport: { width: 1200, height: 900 } });
const probePage = await context.newPage();
await probePage.goto(url);
const approaches = await probePage.evaluate(() => window.listApproaches());
await probePage.close();

for (const name of approaches) {
	if (filter && !name.includes(filter)) continue;
	process.stderr.write(`running ${name} ...\n`);
	const page = await context.newPage();
	page.on('console', (msg) => {
		if (msg.type() === 'error') process.stderr.write(`  [console] ${msg.text()}\n`);
	});
	page.on('pageerror', (err) => process.stderr.write(`  [pageerror] ${err.message}\n`));
	await page.goto(url);
	try {
		const result = await page.evaluate(
			(approach) => window.runBenchmark(approach),
			name,
		);
		await page
			.locator('#map canvas')
			.first()
			.screenshot({ path: join(root, 'screenshots', `${name}.png`) })
			.catch(() => {});
		results.push(result);
		process.stderr.write(`  init ${result.initTotalMs.toFixed(0)}ms, append avg ${result.appendLatency.avg.toFixed(1)}ms\n`);
	} catch (err) {
		results.push({ approach: name, error: String(err).slice(0, 300) });
		process.stderr.write(`  FAILED: ${err}\n`);
	}
	await page.close();
}

await browser.close();
server.close();

await writeFile(join(root, 'results.json'), JSON.stringify(results, null, 2));

const fmt = (v, digits = 0) => (v == null ? '–' : (+v).toFixed(digits));
console.log(
	'| approach | init (ms) | append call avg/p95 (ms) | append latency avg/p95 (ms) | pan fps | zoom fps | fps @appends | recolor (ms) | heap (MB) |',
);
console.log('|---|---|---|---|---|---|---|---|---|');
for (const r of results) {
	if (r.error) {
		console.log(`| ${r.approach} | ERROR: ${r.error} | | | | | | | |`);
		continue;
	}
	console.log(
		`| ${r.approach} | ${fmt(r.initTotalMs)} | ${fmt(r.appendCall.avg, 1)} / ${fmt(r.appendCall.p95, 1)} | ${fmt(r.appendLatency.avg, 1)} / ${fmt(r.appendLatency.p95, 1)} | ${fmt(r.panFps, 1)} | ${fmt(r.zoomFps, 1)} | ${fmt(r.fpsDuringAppends, 1)} | ${fmt(r.recolorTotalMs)} | ${fmt(r.heapMB, 1)} |`,
	);
}
