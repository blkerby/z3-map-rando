import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { defineConfig, type Plugin } from 'vite';

// The settings data (presets, skill classifications, items) is prepared by the randomizer's catalog
// builder from its `data/` directory, as `settings_data.json` in the catalog output directory
// (`build/` by default, see catalog-build.toml). Run the builder whenever this config is loaded, so
// `dev`, `build` and `check` always see current data; unchanged inputs are reused from its cache.
const repoRoot = path.resolve(import.meta.dirname, '..');
const dataDirectory = path.join(repoRoot, 'data');
const settingsData = path.join(repoRoot, 'build', 'settings_data.json');

function prepareSettingsData() {
	execFileSync('cargo', ['run', '--quiet', '--package', 'catalog_builder', '--', 'presets'], {
		cwd: repoRoot,
		stdio: 'inherit'
	});
}

prepareSettingsData();

/** In dev, prepare the settings data again when its sources change; the page then reloads. */
function watchSettingsData(): Plugin {
	return {
		name: 'watch-settings-data',
		configureServer(server) {
			server.watcher.add([dataDirectory, settingsData]);
			server.watcher.on('change', (file) => {
				if (!file.startsWith(dataDirectory + path.sep)) return;
				try {
					prepareSettingsData();
				} catch {
					// The builder has printed its error; the page keeps the last good data.
				}
			});
		}
	};
}

export default defineConfig({
	server: { fs: { allow: [path.dirname(settingsData)] } },
	css: {
		preprocessorOptions: {
			scss: {
				// Bootstrap 5.3 still uses Sass features that Dart Sass is retiring.
				// This hides those warnings; remove it once Bootstrap migrates.
				quietDeps: true,
				silenceDeprecations: ['import', 'global-builtin', 'color-functions', 'if-function']
			}
		}
	},
	plugins: [
		watchSettingsData(),
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// Static files for the randomizer's server to host. The Generate page is client-only (see its +page.ts).
			adapter: adapter(),
			alias: { $settings_data: settingsData }
		})
	]
});
