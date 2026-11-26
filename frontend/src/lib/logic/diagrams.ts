/** Room diagram images from the z3-json-data `roomDiagrams` folders. */
const images = import.meta.glob<string>('$z3_json_data/rooms/**/roomDiagrams/*.png', {
	query: '?url',
	import: 'default',
	eager: true
});

export interface Diagram {
	/** Floor or area, e.g. "1F", "B2", or "Sewers"; "Main" for a dungeon diagram named only after the dungeon. */
	label: string;
	url: string;
}

/** Sort position of a dungeon diagram: basements, then floors from the bottom up, then named areas. */
function floorOrder(label: string): number {
	const basement = /^B(\d)$/.exec(label);
	if (basement) {
		return -Number(basement[1]);
	}
	const floor = /^(\d)F$/.exec(label);
	if (floor) {
		return Number(floor[1]);
	}
	return label === 'Main' ? 0 : 100;
}

/**
 * Diagrams for a room, given its file path relative to the rooms folder. Overworld diagrams share the room
 * file's hexadecimal prefix; dungeon diagrams are named after the dungeon (ignoring apostrophes) and floor.
 */
export function findDiagrams(roomPath: string, roomName: string): Diagram[] {
	const folder = roomPath.slice(0, roomPath.lastIndexOf('/'));
	const fileName = roomPath.slice(roomPath.lastIndexOf('/') + 1);
	const prefix =
		folder === 'overworld' ? fileName.split(' ')[0] + ' ' : roomName.replaceAll("'", '');
	const diagrams: Diagram[] = [];
	for (const [key, url] of Object.entries(images)) {
		const relative = key.slice(key.indexOf('/rooms/') + '/rooms/'.length);
		if (!relative.startsWith(folder + '/roomDiagrams/')) {
			continue;
		}
		const name = relative.slice(relative.lastIndexOf('/') + 1, -'.png'.length).replaceAll("'", '');
		if (folder === 'overworld' && name.startsWith(prefix)) {
			diagrams.push({ label: '', url });
		} else if (name === prefix || name.startsWith(prefix + ' ')) {
			diagrams.push({ label: name.slice(prefix.length).trim() || 'Main', url });
		}
	}
	diagrams.sort(
		(a, b) => floorOrder(a.label) - floorOrder(b.label) || a.label.localeCompare(b.label)
	);
	return diagrams;
}

/** The diagram shown first: 1F where it exists, otherwise B1, otherwise the first. */
export function selectDefaultDiagram(diagrams: Diagram[]): Diagram | undefined {
	return (
		diagrams.find((diagram) => diagram.label === '1F') ??
		diagrams.find((diagram) => diagram.label === 'B1') ??
		diagrams[0]
	);
}
