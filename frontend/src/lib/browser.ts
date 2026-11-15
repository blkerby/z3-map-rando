import wasmUrl from '$browser/browser_bg.wasm?url&no-inline';

let initialized: Promise<typeof import('$browser/browser.js')> | undefined;

export function loadBrowser() {
	initialized ??= import('$browser/browser.js')
		.then(async (browser) => {
			await browser.default({ module_or_path: wasmUrl });
			return browser;
		})
		.catch((error) => {
			initialized = undefined;
			throw error;
		});
	return initialized;
}
