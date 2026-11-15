// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		interface PageState {
			/** Generation notices, passed from /generate to the new seed's page. */
			notices?: string[];
		}
		// interface Platform {}
	}
}

export {};
