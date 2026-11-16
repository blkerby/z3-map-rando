import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { browserPackage, buildDirectory, prepareBrowserPackage, prepareCatalogs, settingsData } from './scripts/prepare.js';
import { defineConfig } from 'vite';

// The frontend reads the settings data, browser package and catalogs that scripts/prepare.js builds, so prepare
// them whenever this config is loaded, and `dev`, `build` and `check` always see current inputs. Vite loads this
// config several times in one process (SvelteKit's build runs a nested Vite build with it), each time as a fresh
// module, so the process environment records that the inputs are already prepared. During `npm run dev`, the dev
// script (scripts/dev.js) prepares them again when the repository changes.
if (!process.env.Z3_FRONTEND_INPUTS_PREPARED) {
	await prepareCatalogs();
	await prepareBrowserPackage();
	process.env.Z3_FRONTEND_INPUTS_PREPARED = '1';
}

export default defineConfig({
	server: {
		fs: { allow: [buildDirectory] },
		proxy: { '/api/': 'http://127.0.0.1:8080' }
	},
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
		sveltekit({
			compilerOptions: {
				// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
				runes: ({ filename }) =>
					filename.split(/[/\\]/).includes('node_modules') ? undefined : true
			},

			// Static files for the randomizer's server to host. The Generate page is client-only (see its +page.ts).
			// Seed pages aren't prerendered; the server answers them with the `spa.html` fallback.
			adapter: adapter({ fallback: 'spa.html' }),
			alias: {
				$settings_data: settingsData,
				$browser: browserPackage,
				// A directory alias, since Vite only matches a file alias exactly or followed by `/`, not by `?url`.
				$build: buildDirectory
			}
		})
	]
});
