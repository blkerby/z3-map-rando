import { error } from '@sveltejs/kit';
import { findNotable, notables } from '$lib/server/logic';

export function entries() {
	const result = [];
	for (const notable of notables) {
		result.push({ roomId: String(notable.room.id), notableId: String(notable.id) });
	}
	return result;
}

export function load({ params }) {
	const notable = findNotable(Number(params.roomId), Number(params.notableId));
	if (!notable) {
		error(404, 'Notable not found');
	}
	return { notable };
}
