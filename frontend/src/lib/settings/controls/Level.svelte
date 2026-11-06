<script lang="ts">
	let {
		levels,
		value,
		label,
		disabled = false,
		onchange
	}: {
		levels: { value: number; label: string }[];
		value: number;
		label: string;
		disabled?: boolean;
		onchange: (value: number) => void;
	} = $props();

	// The slider moves over positions 0..n-1, so levels don't have to be contiguous numbers.
	const index = $derived(Math.max(0, levels.findIndex((l) => l.value === value)));
</script>

<div class="level">
	<input
		type="range"
		class="form-range"
		min="0"
		max={levels.length - 1}
		step="1"
		value={index}
		{disabled}
		aria-label={label}
		aria-valuetext={levels[index]?.label}
		oninput={(e) => onchange(levels[Number(e.currentTarget.value)].value)}
	/>
	<div class="ticks">
		{#each levels as l, i (l.value)}
			<button
				type="button"
				class="tick"
				class:on={i === index}
				{disabled}
				tabindex="-1"
				onclick={() => onchange(l.value)}
			>
				{l.label}
			</button>
		{/each}
	</div>
</div>

<style>
	.level {
		max-width: 28rem;
	}

	.ticks {
		display: flex;
		justify-content: space-between;
		margin-top: 0.1rem;
	}

	.tick {
		padding: 0;
		border: 0;
		background: none;
		color: var(--bs-body-color);
		font-size: 0.78rem;
		opacity: 0.55;
	}

	.tick.on {
		opacity: 1;
		font-weight: 600;
	}

	.tick:disabled {
		cursor: default;
	}
</style>
