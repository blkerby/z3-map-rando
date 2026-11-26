import type { RoomSpace } from '$lib/logic/types';

/** A room ID space in Logic page URLs: `o` (overworld) or `u` (underworld). */
export function match(param: string): param is RoomSpace {
	return param === 'o' || param === 'u';
}
