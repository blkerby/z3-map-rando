// `npm run dev`: the Vite dev server together with the randomizer server, which serves only the API (Vite's
// `/api/` proxy forwards to it). Arguments go to the randomizer server, e.g. `npm run dev -- --store file:seeds`.
//
// Any change in the repository outside output directories rebuilds the server with Cargo, which decides what
// the server depends on (its build script also updates the catalogs), and the browser package. The server
// restarts only if its binary or a catalog changed; the page reloads if the settings data or browser package did.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { createServer } from 'vite';
import { browserPackage, buildDirectory, prepareBrowserPackage, repoRoot, run, settingsData } from './prepare.js';

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

function startServer() {
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
	// Each failing command has printed its error; the last good server and browser package stay in use.
	try {
		const before = stampServerInputs();
		await run('cargo', ['build', '--release', '--package', 'server']);
		if (stampServerInputs() !== before) {
			console.log('Restarting the randomizer server.');
			await restartServer();
		}
	} catch {
		/* see above */
	}
	try {
		await prepareBrowserPackage();
	} catch {
		/* see above */
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
const vite = await createServer();
// Outside Vite's root, so not watched by default. A change reloads the page.
vite.watcher.add([settingsData, browserPackage]);

try {
	await run('cargo', ['build', '--release', '--package', 'server']);
} catch {
	await vite.close();
	process.exit(1);
}
startServer();
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
