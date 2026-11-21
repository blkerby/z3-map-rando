import type { PageLoad } from './$types';

// Seeds are only known at runtime: the server answers /seed/<name>/ with the SPA fallback page. The page
// loads the seed itself, so the navbar shows while it does.
export const ssr = false;
export const prerender = false;

export const load: PageLoad = ({ params }) => {
	return { seed: params.name };
};
