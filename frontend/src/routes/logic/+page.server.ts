import { listRooms } from '$lib/server/logic';

export function load() {
	return { rooms: listRooms() };
}
