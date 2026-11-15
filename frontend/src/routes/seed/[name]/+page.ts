import { error } from '@sveltejs/kit';
import type { Tree } from '$lib/settings/tree';
import type { PageLoad } from './$types';

// Seeds are only known at runtime: the server answers /seed/<name>/ with the SPA fallback page.
export const ssr = false;
export const prerender = false;

export const load: PageLoad = async ({ fetch, params }) => {
	const response = await fetch(`/api/seed/${encodeURIComponent(params.name)}`);
	if (response.status === 404) error(404, `Seed ${params.name} was not found.`);
	if (!response.ok) error(response.status, 'Could not load the seed.');
	const data = await response.json();
	return { seed: params.name, settings: data.settings_source.document as Tree };
};
