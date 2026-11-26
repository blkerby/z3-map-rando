import { difficultyOrder, listRooms, listTechs, listTieredTechs } from '$lib/server/logic';

export function load() {
	return {
		rooms: listRooms(),
		techs: listTechs(),
		tieredTechs: listTieredTechs(),
		difficulties: difficultyOrder
	};
}
