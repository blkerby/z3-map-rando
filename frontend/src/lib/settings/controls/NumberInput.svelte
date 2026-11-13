<script lang="ts">
	let {
		value,
		min,
		max,
		step = 1,
		label,
		disabled = false,
		onchange
	}: {
		value: number;
		min: number;
		max: number;
		step?: number;
		label: string;
		disabled?: boolean;
		onchange: (v: number) => void;
	} = $props();

	// Commit on change (blur/Enter/arrows), so typing "12" isn't clamped at the "1".
	function commit(e: Event & { currentTarget: HTMLInputElement }) {
		const el = e.currentTarget;
		const raw = Number(el.value);
		if (el.value.trim() === '' || !Number.isFinite(raw)) {
			el.value = String(value); // junk: put the old value back
			return;
		}
		const snapped = Math.round(raw / step) * step;
		const next = Math.min(max, Math.max(min, Number(snapped.toFixed(6))));
		el.value = String(next); // show the clamped value even if it equals the old one
		if (next !== value) onchange(next);
	}
</script>

<input
	type="number"
	class="form-control form-control-sm num"
	aria-label={label}
	{min}
	{max}
	{step}
	{value}
	{disabled}
	onchange={commit}
/>

<style>
	.num {
		width: 4.75rem;
		font-variant-numeric: tabular-nums;
	}
</style>
