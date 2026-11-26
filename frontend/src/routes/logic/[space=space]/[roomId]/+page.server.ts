import { error } from '@sveltejs/kit';
import { findRoom, rooms } from '$lib/server/logic';

export function entries() {
	const result = [];
	for (const room of rooms) {
		result.push({ space: room.space, roomId: String(room.id) });
	}
	return result;
}

export function load({ params }) {
	const room = findRoom(params.space, Number(params.roomId));
	if (!room) {
		error(404, 'Room not found');
	}
	return { room };
}
