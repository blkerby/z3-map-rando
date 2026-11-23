// `npm run dev`: the Vite dev server together with the randomizer server, which serves only the API (Vite's
// `/api/` proxy forwards to it). Arguments go to the randomizer server, e.g. `npm run dev -- --store file:seeds`.
//
// Any change in the repository outside output directories rebuilds the server with Cargo, which decides what
// the server depends on (its build script also updates the catalogs), and the browser package. The server
// restarts only if its binary or a catalog changed; the page reloads if the settings data or browser package did.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { createServer } from 'vite';
import { apiTarget, browserPackage, buildDirectory, prepareBrowserPackage, repoRoot, run, settingsData } from './prepare.js';

const serverArgs = ['--api-only', ...process.argv.slice(2)];
const serverBinary = path.join(repoRoot, 'target', 'release', 'z3-map-rando-server');
// The map pool path, as in catalog-build.toml.
const mapPool = path.join(repoRoot, 'maps', 'map_pool.bin');
// Top-level entries not to watch: outputs, the frontend (Vite watches it), and hidden entries such as `.git`.
const unwatched = new Set(['build', 'frontend', 'seeds', 'target']);

/** Modification times of the files the server runs from, to tell whether a rebuild changed any of them. */
function stampServerInputs() {
	const files = [serverBinary, mapPool];
	for (const name of fs.readdirSync(buildDirectory)) if (name.endsWith('.bin')) files.push(path.join(buildDirectory, name));
	const stamps = [];
	for (const file of files) stamps.push(fs.statSync(file, { throwIfNoEntry: false })?.mtimeMs);
	return JSON.stringify(stamps);
}

/** @type {import('node:child_process').ChildProcess | null} */
let server = null;
let serverInputs = '';

function startServer() {
	serverInputs = stampServerInputs();
	server = spawn(serverBinary, serverArgs, { cwd: repoRoot, stdio: ['ignore', 'inherit', 'inherit'] });
}

async function restartServer() {
	if (server && server.exitCode === null && server.signalCode === null) {
		const running = server;
		const exited = new Promise((resolve) => running.once('exit', resolve));
		running.kill();
		await exited;
	}
	startServer();
}

/** Settings data shared by the frontend and the catalog the API loads at startup. */
function readSettingsData() {
	return JSON.parse(fs.readFileSync(settingsData, 'utf8'));
}

/** @param {string} revision @param {number} formatVersion */
async function waitForSettingsRevision(revision, formatVersion) {
	const signal = AbortSignal.timeout(5000);
	while (!signal.aborted) {
		try {
			const response = await fetch(`${apiTarget}/api/settings/process`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ kind: 'generation', document: { format_version: formatVersion } }),
				signal
			});
			if (response.ok && (await response.json()).revision === revision) return;
		} catch {
			// The API may still be starting.
		}
		await delay(100);
	}
	throw new Error('The randomizer API did not load the new settings revision. Skipping the settings reload.');
}

let publishedRevision = '';
let rebuilding = false;
let rebuildAgain = false;
/** @type {NodeJS.Timeout | undefined} */
let rebuildTimer;

async function rebuild() {
	if (rebuilding) {
		rebuildAgain = true;
		return;
	}
	rebuilding = true;
	try {
		await run('cargo', ['build', '--release', '--package', 'server']);
		if (stampServerInputs() !== serverInputs) {
			console.log('Restarting the randomizer server.');
			await restartServer();
		}
		const data = readSettingsData();
		const revision = data.presets.revision;
		if (revision !== publishedRevision) {
			await waitForSettingsRevision(revision, data.format_version);
			for (const environment of Object.values(vite.environments)) environment.moduleGraph.onFileChange(settingsData);
			vite.ws.send({ type: 'full-reload', path: '*' });
			publishedRevision = revision;
		}
	} catch (error) {
		// Cargo prints its own errors.
		if (error instanceof Error) console.error(error.message);
	}
	try {
		await prepareBrowserPackage();
	} catch {
		// The browser package build prints its own errors.
	}
	rebuilding = false;
	if (rebuildAgain) {
		rebuildAgain = false;
		void rebuild();
	}
}

function scheduleRebuild() {
	clearTimeout(rebuildTimer);
	rebuildTimer = setTimeout(rebuild, 200);
}

// Loading the config prepares the catalogs and browser package (see vite.config.ts).
// The generated settings module is invalidated only when the API serves the matching catalog.
const vite = await createServer({ server: { watch: { ignored: [settingsData] } } });
vite.watcher.add(browserPackage);

try {
	await run('cargo', ['build', '--release', '--package', 'server']);
} catch {
	await vite.close();
	process.exit(1);
}
startServer();
publishedRevision = readSettingsData().presets.revision;
process.on('exit', () => server?.kill());
// Exit (running the handler above) on signals too; by default they end Node without it, leaving the server running.
for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(signal, () => process.exit());

fs.watch(repoRoot, (_event, name) => {
	if (name && !name.startsWith('.') && !unwatched.has(name)) scheduleRebuild();
});
for (const entry of fs.readdirSync(repoRoot, { withFileTypes: true })) {
	if (entry.isDirectory() && !entry.name.startsWith('.') && !unwatched.has(entry.name)) {
		fs.watch(path.join(repoRoot, entry.name), { recursive: true }, scheduleRebuild);
	}
}

await vite.listen();
vite.printUrls();
vite.bindCLIShortcuts({ print: true });
