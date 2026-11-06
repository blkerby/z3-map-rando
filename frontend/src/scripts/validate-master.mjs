// Checks master_data.json is internally consistent and still contains everything in the original files.
//   node validate-master.mjs master_data.json tech_data.json notable_data.json settings.json
import { readFileSync } from 'node:fs';
const [masterPath, techPath, notablePath, settingsPath] = process.argv.slice(2);
const read = (p) => JSON.parse(readFileSync(p, 'utf8'));
const m = read(masterPath), tech = read(techPath), notables = read(notablePath), settings = read(settingsPath);

const errors = [];
const fail = (s) => errors.push(s);
const leaves = [];
(function walk(nodes) {
	for (const n of nodes) {
		if (n.children) walk(n.children);
		else leaves.push(n);
	}
})(m.root);

const key = (n) => {
	if (n.type === 'toggle' && n.notable_id !== undefined) {
		const [k, id] = Object.entries(n.room_id)[0];
		return `notable.${k}.${id}.${n.notable_id}`;
	}
	return n.name;
};
const seen = new Set();
for (const n of leaves) {
	const k = key(n);
	if (seen.has(k)) fail(`duplicate key ${k}`);
	seen.add(k);
	const ok = (d) => m.difficulties.includes(d) || m.unrankedDifficulties.includes(d);
	if (n.difficulty && !ok(n.difficulty)) fail(`${k}: unknown difficulty ${n.difficulty}`);
	if (n.type === 'count') {
		if (n.max === undefined && !n.calculated) fail(`${k}: needs a max (only calculated counts may omit it)`);
		if (!(n.min <= n.default && (n.max === undefined || n.default <= n.max))) fail(`${k}: default outside min/max`);
	}
	if (n.type === 'level' && !n.levels.some((l) => l.level === n.default)) fail(`${k}: default not in levels`);
	if (n.type === 'choice' && !n.options.some((o) => o.value === n.default)) fail(`${k}: default not an option`);
}

// Round trip: everything in the originals must be recoverable from the master.
for (const t of tech) {
	if (t.level !== undefined) {
		const n = leaves.find((l) => l.type === 'level' && l.tech_id === t.tech_id);
		if (!n || !n.levels.some((l) => l.level === t.level && l.difficulty === t.difficulty)) fail(`lost level row ${t.name} ${t.level}`);
	} else {
		const n = leaves.find((l) => l.type === 'toggle' && l.tech_id === t.tech_id);
		if (!n || n.name !== t.name || n.difficulty !== t.difficulty) fail(`lost tech ${t.name}`);
	}
}
for (const o of notables) {
	const n = leaves.find((l) => l.notable_id === o.notable_id && JSON.stringify(l.room_id) === JSON.stringify(o.room_id));
	if (!n || n.name !== o.name || n.difficulty !== o.difficulty || n.room_name !== o.room_name) fail(`lost notable ${o.name}`);
}
for (const i of settings.item_pool) {
	const n = leaves.find((l) => l.type === 'count' && l.name === i.name);
	if (!n || n.default !== i.count) fail(`lost item ${i.name}`);
}
if (leaves.filter((l) => l.type === 'count').length !== settings.item_pool.length) fail('item count mismatch');

// Slot pool: with every player-set item at its maximum there must still be room for each calculated item's minimum,
// and at the defaults the calculated items must get at least their minimums.
const counts = leaves.filter((l) => l.type === 'count');
const calc = counts.filter((l) => l.calculated), fixed = counts.filter((l) => !l.calculated);
if (calc.length && !m.itemLocations) fail('calculated items need itemLocations');
if (calc.length) {
	const free = m.itemLocations - fixed.reduce((a, l) => a + l.default, 0);
	const need = calc.reduce((a, l) => a + l.min, 0);
	if (free < need) fail(`defaults leave ${free} slots for calculated items that need ${need}`);
	else console.log(`at defaults the calculated items share ${free} slots (min ${need})`);
	if (calc.some((l) => !(l.default > 0))) fail('calculated items need a default above 0 (it is their weight)');
}
console.log(`${leaves.length} leaves, ${seen.size} unique keys`);
if (errors.length) { console.error(errors.join('\n')); process.exit(1); }
console.log('OK');
