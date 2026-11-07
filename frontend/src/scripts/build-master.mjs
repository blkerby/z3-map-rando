// One-time bootstrap: builds master_data.json from the three original files.
// After this, master_data.json is the source of truth; edit it directly.
//   node build-master.mjs tech_data.json notable_data.json settings.json master_data.json
import { readFileSync, writeFileSync } from 'node:fs';

const [techPath, notablePath, settingsPath, outPath] = process.argv.slice(2);
const tech = JSON.parse(readFileSync(techPath, 'utf8'));
const notables = JSON.parse(readFileSync(notablePath, 'utf8'));
const settings = JSON.parse(readFileSync(settingsPath, 'utf8'));

const difficulties = ['Basic', 'Medium', 'Hard', 'Very Hard', 'Expert'];
const unrankedDifficulties = ['Ignored'];
const rank = (d) => (difficulties.includes(d) ? difficulties.indexOf(d) : 99);

// ---- Logic: proficiencies (rows with a `level`) and tech (rows without) ----
const profRows = tech.filter((t) => t.level !== undefined);
const techRows = tech.filter((t) => t.level === undefined);

// Slider text that differs from the community difficulty name (the difficulty itself is kept as-is).
const LEVEL_LABELS = { darkProficiency: { 0: 'None' } };

const profNodes = [...new Set(profRows.map((r) => r.tech_id))].map((id) => {
	const rows = profRows.filter((r) => r.tech_id === id).sort((a, b) => a.level - b.level);
	return {
		type: 'level',
		tech_id: id,
		name: rows[0].name,
		levels: rows.map((r) => ({
			level: r.level,
			difficulty: r.difficulty,
			...(LEVEL_LABELS[r.name]?.[r.level] ? { label: LEVEL_LABELS[r.name][r.level] } : {})
		})),
		default: rows[0].level
	};
});

const IGNORED_HIDDEN = true; // unranked tech is hidden; its default is its fixed value
const techNodes = [...techRows]
	.sort((a, b) => rank(a.difficulty) - rank(b.difficulty) || a.tech_id - b.tech_id)
	.map((t) => ({
		type: 'toggle',
		tech_id: t.tech_id,
		name: t.name,
		difficulty: t.difficulty,
		...(IGNORED_HIDDEN && unrankedDifficulties.includes(t.difficulty) ? { hidden: true } : {}),
		default: false
	}));

// Notables stay flat: the page shows the room in each label and groups them under each difficulty.
const notableNodes = [...notables]
	.sort((a, b) => Object.values(a.room_id)[0] - Object.values(b.room_id)[0] || a.notable_id - b.notable_id)
	.map((n) => ({
		type: 'toggle',
		room_id: n.room_id,
		room_name: n.room_name,
		notable_id: n.notable_id,
		name: n.name,
		difficulty: n.difficulty,
		default: false
	}));

// ---- Items: counts come from settings.json; labels/categories/limits are PLACEHOLDERS ----
// [label, category, max, min = 0]. max for one-of items is their vanilla count; fillers allow up to double.
const META = {
	FireRod: ['Fire Rod', 'Equipment', 1], IceRod: ['Ice Rod', 'Equipment', 1], Hammer: ['Hammer', 'Equipment', 1],
	Hookshot: ['Hookshot', 'Equipment', 1], Bow: ['Bow', 'Equipment', 1], Boomerang: ['Boomerang', 'Equipment', 1],
	MagicPowder: ['Magic Powder', 'Equipment', 1], Bombos: ['Bombos', 'Equipment', 1], Ether: ['Ether', 'Equipment', 1],
	Quake: ['Quake', 'Equipment', 1], Lamp: ['Lamp', 'Equipment', 1], Shovel: ['Shovel', 'Equipment', 1],
	CaneOfSomaria: ['Cane of Somaria', 'Equipment', 1], CaneOfByrna: ['Cane of Byrna', 'Equipment', 1],
	Cape: ['Magic Cape', 'Equipment', 1], MagicMirror: ['Magic Mirror', 'Equipment', 1],
	BookOfMudora: ['Book of Mudora', 'Equipment', 1], Flippers: ['Flippers', 'Equipment', 1],
	MoonPearl: ['Moon Pearl', 'Equipment', 1], BugCatchingNet: ['Bug-Catching Net', 'Equipment', 1],
	Mushroom: ['Mushroom', 'Equipment', 1], Bottle: ['Bottle', 'Equipment', 4], PegasusBoots: ['Pegasus Boots', 'Equipment', 1],
	Ocarina: ['Ocarina', 'Equipment', 1],
	Sword: ['Sword', 'Upgrades', 4, 1], Shield: ['Shield', 'Upgrades', 3], Glove: ['Glove', 'Upgrades', 2],
	Mail: ['Mail', 'Upgrades', 2], HalfMagic: ['Half Magic', 'Upgrades', 1], Arrow: ['Silver Arrow', 'Upgrades', 1],
	ThreeBombs: ['Bombs (3)', 'Ammo', 34], TenArrows: ['Arrows (10)', 'Ammo', 22],
	OneRupee: ['1 Rupee', 'Rupees', 4], FiveRupees: ['5 Rupees', 'Rupees', 8], TwentyRupees: ['20 Rupees', 'Rupees', 58],
	FiftyRupees: ['50 Rupees', 'Rupees', 14], OneHundredRupees: ['100 Rupees', 'Rupees', 2],
	ThreeHundredRupees: ['300 Rupees', 'Rupees', 10],
	BossHeartContainer: ['Boss Heart Container', 'Hearts', 20], SanctuaryHeartContainer: ['Sanctuary Heart Container', 'Hearts', 2],
	PieceOfHeart: ['Piece of Heart', 'Hearts', 48]
};
const categories = ['Equipment', 'Upgrades', 'Ammo', 'Rupees', 'Hearts'];
const itemGroups = categories.map((cat) => ({
	type: 'group',
	title: cat,
	children: settings.item_pool
		.filter((i) => META[i.name]?.[1] === cat)
		.map((i) => ({
			type: 'count',
			name: i.name,
			label: META[i.name][0],
			default: i.count,
			min: META[i.name][3] ?? 0,
			max: META[i.name][2]
		}))
}));
const unmapped = settings.item_pool.filter((i) => !META[i.name]).map((i) => i.name);
if (unmapped.length) throw new Error('No metadata for items: ' + unmapped.join(', '));

const master = {
	version: 1,
	difficulties,
	unrankedDifficulties,
	root: [
		{
			type: 'section', id: 'logic', title: 'Logic', icon: 'speedometer2',
			children: [
				{ type: 'group', title: 'Proficiencies', children: profNodes },
				// Regrouped on the page as: <difficulty> > Tech / Notables. The title here is only a name.
				{
					type: 'group', title: 'Tech and notables', layout: 'compact', groupBy: 'difficulty',
					children: [
						{ type: 'group', title: 'Tech', children: techNodes },
						{ type: 'group', title: 'Notables', children: notableNodes }
					]
				}
			]
		},
		{
			type: 'section', id: 'map', title: 'Map', icon: 'map',
			children: [
				{
					type: 'choice', name: 'mapLayout', label: 'Map layout', default: 'randomized', display: 'cards',
					options: [
						{ value: 'randomized', label: 'Randomized', desc: 'The map is shuffled.' },
						{ value: 'vanilla', label: 'Vanilla', desc: 'The original map, unchanged.' }
					]
				}
			]
		},
		{ type: 'section', id: 'items', title: 'Items', icon: 'box-seam', layout: 'grid', collapsible: true, collapsed: true, children: itemGroups }
	]
};
writeFileSync(outPath, JSON.stringify(master, null, 2) + '\n');
console.log('wrote', outPath);
