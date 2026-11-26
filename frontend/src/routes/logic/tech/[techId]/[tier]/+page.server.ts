import { error } from '@sveltejs/kit';
import { findTieredTech, tieredTechPages } from '$lib/server/logic';

export function entries() {
	const result = [];
	for (const tech of tieredTechPages) {
		result.push({ techId: String(tech.id), tier: String(tech.tier) });
	}
	return result;
}

export function load({ params }) {
	const tech = findTieredTech(Number(params.techId), Number(params.tier));
	if (!tech) {
		error(404, 'Tech not found');
	}
	return { tech };
}
