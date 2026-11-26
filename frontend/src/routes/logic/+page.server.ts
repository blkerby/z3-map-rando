import {
	difficultyOrder,
	listNotables,
	listRooms,
	listTechs,
	listTieredTechs
} from '$lib/server/logic';

export function load() {
	return {
		rooms: listRooms(),
		techs: listTechs(),
		tieredTechs: listTieredTechs(),
		notables: listNotables(),
		difficulties: difficultyOrder
	};
}
