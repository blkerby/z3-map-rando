/**
 * Settings are nested objects in the shape of `data/settings.json`: scalar values at the leaves, and a
 * setting is identified by its path (e.g. ['skill', 'tech', '5']). Paths are never joined into one
 * persisted key; `pathKey` exists only for in-page lookups and element ids.
 */

export type Value = string | boolean | number;
export interface Tree {
	[key: string]: Value | Tree;
}
export type Path = string[];

export function isTree(x: unknown): x is Tree {
	return typeof x === 'object' && x !== null && !Array.isArray(x);
}

/** The entry at `path`, or undefined. Only own properties are followed, so untrusted input can't reach prototypes. */
export function getAt(tree: unknown, path: Path): unknown {
	let node = tree;
	for (const key of path) {
		if (!isTree(node) || !Object.hasOwn(node, key)) return undefined;
		node = node[key];
	}
	return node;
}

/** Set the entry at `path`, creating objects along the way. */
export function setAt(tree: Tree, path: Path, value: Value | Tree) {
	let node = tree;
	for (const key of path.slice(0, -1)) {
		if (!isTree(node[key])) node[key] = {};
		node = node[key] as Tree;
	}
	node[path[path.length - 1]] = value;
}

/** Paths of every leaf (scalar value) in a tree, depth-first in property order. */
export function leafPaths(tree: Tree, prefix: Path = []): Path[] {
	const out: Path[] = [];
	for (const [key, v] of Object.entries(tree)) {
		if (isTree(v)) out.push(...leafPaths(v, [...prefix, key]));
		else out.push([...prefix, key]);
	}
	return out;
}

/** Is `path` equal to, or inside, `prefix`? */
export function startsWith(path: Path, prefix: Path): boolean {
	for (let i = 0; i < prefix.length; i++) if (path[i] !== prefix[i]) return false;
	return path.length >= prefix.length;
}

/** A string for a path, for in-page lookups and element ids only. */
export function pathKey(path: Path): string {
	return path.join('/');
}

export function cloneTree(tree: Tree): Tree {
	return structuredClone(tree);
}
