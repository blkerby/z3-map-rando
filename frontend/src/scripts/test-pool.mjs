// Checks the filler split rule. Run from the project root:
//   node --experimental-strip-types scripts/test-pool.mjs
import { splitFiller, expectedSplit, adjustSplit } from '../src/lib/master/pool.ts';

// Small seeded generator so the test is repeatable.
function mulberry32(seed) {
	return () => {
		seed |= 0; seed = (seed + 0x6d2b79f5) | 0;
		let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}
const items = [
	{ name: 'ThreeBombs', weight: 17, min: 1 },
	{ name: 'TenArrows', weight: 11, min: 1 },
	{ name: 'OneRupee', weight: 2, min: 1 },
	{ name: 'FiveRupees', weight: 4, min: 1 }
];
let failures = 0;
const check = (ok, msg) => { if (!ok) { failures++; console.error('FAIL:', msg); } else console.log('ok  :', msg); };

for (const slots of [4, 5, 33, 34, 120, 148]) {
	let allSum = true, allMin = true;
	const sums = items.map(() => 0);
	const N = 5000;
	const rand = mulberry32(slots);
	for (let t = 0; t < N; t++) {
		const r = splitFiller(slots, items, rand);
		if (r.reduce((a, b) => a + b, 0) !== slots) allSum = false;
		if (r.some((v, i) => v < items[i].min)) allMin = false;
		r.forEach((v, i) => (sums[i] += v));
	}
	check(allSum, `slots=${slots}: always adds up to ${slots}`);
	check(allMin, `slots=${slots}: every item gets at least its minimum`);
	const extra = slots - 4, totalW = 34;
	const expected = items.map((i) => 1 + (extra * i.weight) / totalW);
	const close = sums.every((s, i) => Math.abs(s / N - expected[i]) < 0.06 * Math.max(1, expected[i]));
	check(close, `slots=${slots}: average split ${sums.map((s) => (s / N).toFixed(1)).join(' / ')} matches weights (expected ${expected.map((e) => e.toFixed(1)).join(' / ')})`);
}
check(JSON.stringify(splitFiller(33, items, mulberry32(7))) === JSON.stringify(splitFiller(33, items, mulberry32(7))), 'same seed gives the same split');
let threw = false; try { splitFiller(3, items, Math.random); } catch { threw = true; }
check(threw, 'fewer slots than the minimums is an error');
const capped = [{ name: 'a', weight: 1, min: 1, max: 3 }, { name: 'b', weight: 1, min: 1, max: 3 }];
check(JSON.stringify(splitFiller(6, capped, mulberry32(1))) === '[3,3]', 'maximums are respected');
threw = false; try { splitFiller(7, capped, Math.random); } catch { threw = true; }
check(threw, 'more slots than the maximums allow is an error');

// expectedSplit: deterministic, adds up, respects minimums
for (const slots of [4, 5, 33, 36, 100]) {
	const e = expectedSplit(slots, items);
	check(e.reduce((a, b) => a + b, 0) === slots && e.every((v, i) => v >= items[i].min), `expectedSplit(${slots}) = ${e.join('/')}`);
}
check(expectedSplit(33, items).join() === '16,10,3,4', 'expectedSplit(33) is 16/10/3/4');

// adjustSplit: sticky and always adds up
{
	const rand = mulberry32(99);
	let cur = expectedSplit(33, items), ok = true, sticky = true;
	for (let t = 0; t < 3000; t++) {
		const target = 4 + Math.floor(rand() * 140);
		const before = cur, next = adjustSplit(before, target, items, rand);
		if (next.reduce((a, b) => a + b, 0) !== target || next.some((v, i) => v < items[i].min)) ok = false;
		// only moves in one direction: slots added never make any item smaller, and vice versa
		const grew = target >= before.reduce((a, b) => a + b, 0);
		if (next.some((v, i) => (grew ? v < before[i] : v > before[i]))) sticky = false;
		cur = next;
	}
	check(ok, 'adjustSplit: always adds up and respects minimums over 3000 random changes');
	check(sticky, 'adjustSplit: adding slots only raises amounts, removing only lowers them');
	const one = adjustSplit([16, 10, 3, 4], 34, items, mulberry32(3));
	check(one.reduce((a, b) => a + b, 0) === 34 && one.filter((v, i) => v !== [16, 10, 3, 4][i]).length === 1, 'one extra slot changes exactly one item');
}
process.exit(failures ? 1 : 0);
