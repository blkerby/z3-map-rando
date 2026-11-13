import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	// The settings data is bundled from the randomizer's `data/` directory, outside this one.
	server: { fs: { allow: ['../data'] } },
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
			adapter: adapter(),
			alias: { $data: '../data' }
		})
	]
});