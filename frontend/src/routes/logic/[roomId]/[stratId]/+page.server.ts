import { error } from '@sveltejs/kit';
import { findRoom, rooms } from '$lib/server/logic';

export function entries() {
	const result = [];
	for (const room of rooms) {
		for (const strat of room.strats) {
			result.push({ roomId: String(room.id), stratId: String(strat.id) });
		}
	}
	return result;
}

export function load({ params }) {
	const room = findRoom(Number(params.roomId));
	const strat = room?.strats.find((strat) => strat.id === Number(params.stratId));
	if (!room || !strat) {
		error(404, 'Strat not found');
	}
	return { room: { ...room, connections: [], strats: [strat] } };
}
