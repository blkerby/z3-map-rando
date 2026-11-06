/**
 * How the leftover item slots are split between the calculated items.
 *
 * The PAGE draws the split (so the player sees the real amounts) and sends it with the other
 * settings; the amounts are saved in share links, so whoever opens a link gets the same numbers.
 * The server must still check them: every amount between its min and max, and the amounts adding
 * up to itemLocations minus the player's own items.
 *
 * Three helpers, no imports, so the server can port them as-is (tested by scripts/test-pool.mjs):
 *  - expectedSplit: the average outcome, used as the starting split (the same every time, so the
 *    first render matches between server and browser)
 *  - adjustSplit:   keeps an existing split "sticky": when the slots change, only the difference is
 *    added to / taken from randomly chosen items
 *  - splitFiller:   a completely fresh random draw (the Reroll button)
 *
 * Draw rule (splitFiller): every calculated item first gets its `min`. The remaining slots are then dealt out one at
 * a time, each to an item chosen at random with probability proportional to its weight (the
 * vanilla amount). An item that has reached its `max` drops out of the draw.
 */

export interface FillerItem {
	name: string;
	/** Relative share (the vanilla amount). Must be above 0. */
	weight: number;
	min: number;
	max?: number;
}

/**
 * @param slots  item slots left for the calculated items (itemLocations minus the player's items)
 * @param rand   returns a number in [0, 1); pass a seeded generator
 * @returns      the amount for each item, in the same order; always adds up to `slots`
 */
export function splitFiller(slots: number, items: FillerItem[], rand: () => number): number[] {
	const out = items.map((i) => i.min);
	let left = slots - out.reduce((a, b) => a + b, 0);
	if (left < 0) throw new Error(`Only ${slots} slots for ${items.length} items that need at least ${slots - left} in total`);

	while (left > 0) {
		// Items that can still take one more.
		const open = items.map((_, i) => i).filter((i) => items[i].max === undefined || out[i] < items[i].max!);
		if (open.length === 0) throw new Error('The calculated items cannot hold that many slots');
		const total = open.reduce((a, i) => a + items[i].weight, 0);
		let pick = rand() * total;
		let chosen = open[open.length - 1];
		for (const i of open) {
			pick -= items[i].weight;
			if (pick < 0) {
				chosen = i;
				break;
			}
		}
		out[chosen]++;
		left--;
	}
	return out;
}

/**
 * The average outcome of splitFiller, rounded so the amounts add up to `slots`. Deterministic.
 * (Ignores `max`: no calculated item has one today.)
 */
export function expectedSplit(slots: number, items: FillerItem[]): number[] {
	const minSum = items.reduce((a, i) => a + i.min, 0);
	const extra = Math.max(0, slots - minSum);
	const totalWeight = items.reduce((a, i) => a + i.weight, 0);
	const exact = items.map((i) => i.min + (extra * i.weight) / totalWeight);
	const out = exact.map(Math.floor);
	let left = Math.max(slots, minSum) - out.reduce((a, b) => a + b, 0);
	// Slots lost to rounding go to the largest remainders (ties: earlier item first).
	const order = exact.map((e, i) => i).sort((a, b) => exact[b] - Math.floor(exact[b]) - (exact[a] - Math.floor(exact[a])) || a - b);
	for (const i of order) {
		if (left <= 0) break;
		out[i]++;
		left--;
	}
	return out;
}

function pickIndex(weights: number[], rand: () => number): number {
	const total = weights.reduce((a, b) => a + b, 0);
	let pick = rand() * total;
	for (let i = 0; i < weights.length; i++) {
		pick -= weights[i];
		if (pick < 0 && weights[i] > 0) return i;
	}
	return weights.findLastIndex((w) => w > 0);
}

/**
 * Change an existing split to add up to `slots`, touching as little as possible: missing slots are
 * dealt out one at a time (weighted by `weight`, skipping items at their max); surplus slots are
 * taken one at a time from items above their min (weighted by weight x how far above min they are).
 * Amounts are first clamped to min/max. If it can't reach `slots` it gets as close as it can.
 */
export function adjustSplit(current: number[], slots: number, items: FillerItem[], rand: () => number): number[] {
	const out = current.map((v, i) => Math.min(items[i].max ?? Infinity, Math.max(items[i].min, Math.round(v) || 0)));
	let diff = slots - out.reduce((a, b) => a + b, 0);
	while (diff > 0) {
		const weights = items.map((it, i) => (it.max === undefined || out[i] < it.max ? it.weight : 0));
		if (weights.every((w) => w === 0)) break;
		out[pickIndex(weights, rand)]++;
		diff--;
	}
	while (diff < 0) {
		const weights = items.map((it, i) => it.weight * (out[i] - it.min));
		if (weights.every((w) => w <= 0)) break;
		out[pickIndex(weights, rand)]--;
		diff++;
	}
	return out;
}
