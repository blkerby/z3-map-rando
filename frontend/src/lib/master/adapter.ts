import type { Item, Preset, Section, Setting, Value } from '$lib/settings/types';
import { isLeaf, labelOf, nodeKey, type LeafNode, type MasterData, type MasterNode, type SectionNode } from './master';

export interface Schema {
	/** What the settings store renders: the top-level settings and sections (cards), as in the master file. */
	items: Item[];
	/** Full-settings presets: "Default" (the schema defaults), then any hand-written ones from the master file. */
	presets: Preset[];
	/** Hidden settings and their fixed values. Not shown, not in links, but sent to the server. */
	hidden: Record<string, Value>;
}

/** Display hints a node passes down to everything below it. */
interface Look {
	layout?: 'list' | 'grid' | 'compact';
	/** Sub-headings already say the difficulty, so rows don't repeat it as a tag. */
	hideBadge?: boolean;
}

/** A node's own display hints win over what it inherited. */
function inherit(n: { layout?: 'list' | 'grid' | 'compact' }, look: Look): Look {
	return { ...look, layout: n.layout ?? look.layout };
}

/**
 * Turn the master file into the settings and presets the settings store understands. A section with
 * settings of its own gets its own presets: those listed in the master file, otherwise one per
 * difficulty if its settings have difficulties (Logic), otherwise just "Default".
 */
export function buildSchema(master: MasterData): Schema {
	const hidden: Record<string, Value> = {};
	let leaves: LeafNode[] = []; // visible leaves of the current section, for preset derivation
	const seen = new Set<string>();

	function toSetting(n: LeafNode, key: string, path: string[], look: Look): Setting {
		const heading = path.length ? path.join(' › ') : undefined;
		const base = { id: key, label: labelOf(n), help: n.desc, notes: n.notes, heading, headingPath: path.length ? path : undefined, layout: look.layout };
		switch (n.type) {
			case 'toggle': {
				const ranked = n.difficulty && master.difficulties.includes(n.difficulty);
				return { ...base, kind: 'toggle', default: n.default, badge: ranked && !look.hideBadge ? n.difficulty : undefined };
			}
			case 'count':
				return { ...base, kind: 'number', display: 'input', min: n.min, max: master.itemLocations, step: n.step ?? 1, default: n.default, poolItem: n.poolItem };
			case 'choice':
				return {
					...base,
					kind: 'choice',
					display: n.display,
					default: n.default,
					options: n.options.map((o) => ({ value: o.value, label: o.label ?? o.value, desc: o.desc }))
				};
			case 'level':
				return {
					...base,
					kind: 'level',
					default: n.default,
					levels: n.levels.map((l) => ({ value: l.level, label: l.label ?? l.difficulty }))
				};
		}
	}

	function collect(nodes: MasterNode[], path: string[], out: Item[], sectionTitle: string, look: Look) {
		for (const n of nodes) {
			if (n.type === 'section') {
				const section = toSection(n);
				if (section.items.length) out.push(section);
			} else if (n.type === 'group' && n.groupBy === 'difficulty') {
				// One heading per difficulty in ladder order (anything off the ladder goes last). Under each,
				// the group's own leaves, then each child group's leaves of that difficulty under its title.
				const next = { ...inherit(n, look), hideBadge: true };
				const leavesHere = n.children.filter(isLeaf);
				const childGroups = n.children.filter((c) => c.type === 'group');
				const diffOf = (l: LeafNode) => ('difficulty' in l ? l.difficulty : undefined);
				const everyLeaf = [...leavesHere, ...childGroups.flatMap((g) => g.children.filter(isLeaf))];
				const known = [...master.difficulties, ...master.unrankedDifficulties];
				const extra = [...new Set(everyLeaf.map(diffOf).filter((d): d is string => !!d && !known.includes(d)))];
				collect(leavesHere.filter((l) => !diffOf(l)), path, out, sectionTitle, inherit(n, look));
				for (const d of [...known, ...extra]) {
					collect(leavesHere.filter((l) => diffOf(l) === d), [...path, d], out, sectionTitle, next);
					for (const g of childGroups) {
						const mine = g.children.filter(isLeaf).filter((l) => diffOf(l) === d);
						collect(mine, n.subheadings === false ? [...path, d] : [...path, d, g.title], out, sectionTitle, inherit(g, next));
					}
				}
			} else if (n.type === 'group') {
				collect(n.children, [...path, n.title], out, sectionTitle, inherit(n, look));
			} else if (['toggle', 'count', 'choice', 'level'].includes(n.type)) {
				const key = nodeKey(n);
				if (seen.has(key)) throw new Error(`master_data: duplicate setting key "${key}"`);
				seen.add(key);
				if (n.hidden) {
					hidden[key] = n.default;
					continue;
				}
				leaves.push(n);
				// A heading that just repeats the card title adds nothing.
				const shown = path.length === 1 && path[0] === sectionTitle ? [] : path;
				out.push(toSetting(n, key, shown, look));
			}
			// Unknown node types are skipped on purpose: a newer server must not break this page.
		}
	}

	function toSection(n: SectionNode): Section {
		const outer = leaves;
		leaves = [];
		const items: Item[] = [];
		collect(n.children, [], items, n.title, inherit(n, {}));
		const hasSettings = items.some((item) => item.kind !== 'section');
		const presets = hasSettings
			? (n.presets ?? buildDifficultyPresets(master, leaves) ?? [{ id: 'default', name: 'Default', values: {} }])
			: undefined;
		leaves = outer;
		const pool = n.itemPool ? { total: master.itemLocations } : undefined;
		return { kind: 'section', id: n.id, title: n.title, icon: n.icon, collapsible: n.collapsible, collapsed: n.collapsed, pool, presets, items };
	}

	const items: Item[] = [];
	collect(master.root, [], items, '', {});

	const presets: Preset[] = [{ id: 'default', name: 'Default', values: {} }];
	for (const p of master.presets ?? []) presets.push({ id: p.id, name: p.name, values: p.values });
	return { items, presets, hidden };
}

function slug(s: string): string {
	return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

/**
 * One preset per rung of the difficulty ladder. A preset at difficulty D turns on every toggle
 * ranked at or below D and sets each proficiency to its highest level ranked at or below D.
 * Unranked difficulties (e.g. "Ignored") never take part. Undefined when the leaves have no
 * difficulties at all. The first rung should match the defaults, since it is selected by default.
 */
function buildDifficultyPresets(master: MasterData, leaves: LeafNode[]): Preset[] | undefined {
	const rank = (d: string | undefined) => (d ? master.difficulties.indexOf(d) : -1);
	const presets: Preset[] = [];
	let any = false;

	master.difficulties.forEach((name, i) => {
		const values: Record<string, Value> = {};
		for (const n of leaves) {
			const key = nodeKey(n);
			if (n.type === 'toggle') {
				const r = rank(n.difficulty);
				if (r >= 0) any = true;
				if (r >= 0 && r <= i && n.default !== true) values[key] = true;
			} else if (n.type === 'level') {
				const reachable = n.levels.filter((l) => {
					const r = rank(l.difficulty);
					return r >= 0 && r <= i;
				});
				if (reachable.length) {
					any = true;
					const top = Math.max(...reachable.map((l) => l.level));
					if (top !== n.default) values[key] = top;
				}
			}
		}
		presets.push({ id: `difficulty-${slug(name)}`, name, values });
	});

	return any ? presets : undefined;
}
