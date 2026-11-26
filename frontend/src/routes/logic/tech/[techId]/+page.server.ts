import { error } from '@sveltejs/kit';
import { findTech, techs } from '$lib/server/logic';

export function entries() {
	const result = [];
	for (const tech of techs) {
		result.push({ techId: String(tech.id) });
	}
	return result;
}

export function load({ params }) {
	const tech = findTech(Number(params.techId));
	if (!tech) {
		error(404, 'Tech not found');
	}
	return { tech };
}
