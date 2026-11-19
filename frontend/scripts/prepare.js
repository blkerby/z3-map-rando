// Inputs the frontend needs from the Rust side, shared by vite.config.ts and the dev script.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

export const repoRoot = path.resolve(import.meta.dirname, '..', '..');
// The catalog builder's output directory (`output_directory` in catalog-build.toml).
export const buildDirectory = path.join(repoRoot, 'build');
export const settingsData = path.join(buildDirectory, 'settings_data.json');
export const browserPackage = path.join(buildDirectory, 'browser');
const browserWasm = path.join(repoRoot, 'target', 'wasm32-unknown-unknown', 'release', 'browser.wasm');

/**
 * Run a command from the repository root, showing its output. Rejects if it fails.
 * @param {string} command
 * @param {string[]} args
 * @returns {Promise<void>}
 */
export function run(command, args) {
	return new Promise((resolve, reject) => {
		const child = spawn(command, args, { cwd: repoRoot, stdio: ['ignore', 'inherit', 'inherit'] });
		child.on('error', reject);
		child.on('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${command} failed (${code})`))));
	});
}

/**
 * Prepare the settings data and patch catalog used by the frontend, and the catalogs
 * loaded by the server. Unchanged inputs are reused from the builder's cache.
 */
export async function prepareCatalogs() {
	await run('cargo', ['run', '--quiet', '--package', 'catalog_builder', '--', 'all']);
}

/**
 * Build the browser package (WebAssembly) from `crates/browser`. wasm-pack's slow step (wasm-opt) reruns even
 * when nothing changed, so first run the same Cargo build that wasm-pack does, which leaves its output untouched
 * when up to date, and package only if that output is newer than the package.
 */
export async function prepareBrowserPackage() {
	await run('cargo', ['build', '--quiet', '--lib', '--release', '--target', 'wasm32-unknown-unknown', '--package', 'browser']);
	const packaged = fs.statSync(path.join(browserPackage, 'browser_bg.wasm'), { throwIfNoEntry: false });
	if (!packaged || fs.statSync(browserWasm).mtimeMs > packaged.mtimeMs) {
		await run('wasm-pack', ['build', 'crates/browser', '--target', 'web', '--out-dir', browserPackage]);
	}
}
