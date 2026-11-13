<script lang="ts">
	let {
		value,
		min,
		max,
		step,
		unit = '',
		label,
		disabled = false,
		onchange
	}: {
		value: number;
		min: number;
		max: number;
		step: number;
		unit?: string;
		label: string;
		disabled?: boolean;
		onchange: (v: number) => void;
	} = $props();
</script>

<div class="d-flex align-items-center gap-2 slider">
	<input
		type="range"
		class="form-range"
		aria-label={label}
		{min}
		{max}
		{step}
		{value}
		{disabled}
		oninput={(e) => onchange(Number(e.currentTarget.value))}
	/>
	<span class="readout">{value}{unit}</span>
</div>

<style>
	.slider {
		width: 100%;
		max-width: 20rem;
		min-width: 0;
	}

	.slider input[type='range'] {
		flex: 1 1 0;
		min-width: 0;
	}

	/* Locked: Bootstrap greys the thumb out completely; keep it in the accent colour so the value stands out. */
	.form-range:disabled {
		opacity: 0.8;
	}

	.form-range:disabled::-webkit-slider-thumb {
		background-color: var(--bs-primary);
	}

	.form-range:disabled::-moz-range-thumb {
		background-color: var(--bs-primary);
	}

	.readout {
		flex: none;
		min-width: 3rem;
		font-variant-numeric: tabular-nums;
		text-align: right;
	}
</style>
