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

	// Half of Bootstrap's 1rem range thumb: the thumb's center, and so each stop, lies between this
	// inset from the left and right edges. Labels are positioned (in CSS) and pointer positions are
	// mapped (here) by the same rule.
	const THUMB_HALF_REM = 0.5;

	let slider = $state<HTMLInputElement>();

	// The stop nearest the pointer while it is over the box, shown as a faint thumb.
	let hoverIndex = $state<number | null>(null);

	function findNearest(e: PointerEvent): number {
		const rect = slider!.getBoundingClientRect();
		const inset = THUMB_HALF_REM * parseFloat(getComputedStyle(document.documentElement).fontSize);
		const fraction = (e.clientX - rect.left - inset) / (rect.width - 2 * inset);
		return Math.min(levels.length - 1, Math.max(0, Math.round(fraction * (levels.length - 1))));
	}

	function selectNearest(e: PointerEvent) {
		const i = findNearest(e);
		if (i !== index) onchange(levels[i].value);
	}

	// Stop positions, shared by the tick labels and the hover thumb.
	function positionOf(i: number): string {
		return `calc(var(--thumb-half) + (100% - 2 * var(--thumb-half)) * ${i / Math.max(1, levels.length - 1)})`;
	}

	// Pressing anywhere in the box outside the native slider selects the nearest stop, and dragging
	// continues to follow the pointer. Presses on the slider itself keep the browser's own handling.
	function onpointerdown(e: PointerEvent) {
		const box = e.currentTarget as HTMLElement;
		if (disabled || e.button !== 0 || e.target instanceof HTMLInputElement) return;
		e.preventDefault();
		box.setPointerCapture(e.pointerId);
		selectNearest(e);
	}

	function onpointermove(e: PointerEvent) {
		const box = e.currentTarget as HTMLElement;
		hoverIndex = findNearest(e);
		if (box.hasPointerCapture(e.pointerId)) selectNearest(e);
	}
</script>

<div
	class="level"
	role="presentation"
	style:--thumb-half="{THUMB_HALF_REM}rem"
	{onpointerdown}
	{onpointermove}
	onpointerleave={() => (hoverIndex = null)}
>
	<div class="track">
		<input
			bind:this={slider}
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
		{#if hoverIndex !== null && hoverIndex !== index && !disabled}
			<span class="hover-thumb" style:left={positionOf(hoverIndex)} aria-hidden="true"></span>
		{/if}
	</div>
	<div class="ticks" aria-hidden="true">
		{#each levels as l, i (l.value)}
			<span class="tick" class:on={i === index} style:left={positionOf(i)}>
				{l.label}
			</span>
		{/each}
	</div>
</div>

<style>
	/* Fixed width, since it sits beside its label in a column sized to its content. */
	.level {
		width: 12rem;
		/* Reach out over the surrounding row padding (without shifting the layout), so presses just
		   past the end stops still land in the box. */
		box-sizing: content-box;
		padding: 0.3rem 0.5rem;
		margin: -0.3rem -0.5rem;
		/* Let horizontal drags reach the pointer handlers on touch screens, while still allowing
		   vertical page scrolling. */
		touch-action: pan-y;
		user-select: none;
	}

	/* When locked, the not-allowed cursor from the lockable wrapper (see theme.scss) shows through. */
	.level:not(:has(:disabled)) {
		cursor: pointer;
	}

	/* The browser's own stylesheet gives range inputs `cursor: default`. */
	.level input {
		cursor: inherit;
	}

	.track {
		position: relative;
	}

	/* Same size as Bootstrap's range thumb, centered on its 1.5rem-tall range input. */
	.hover-thumb {
		position: absolute;
		top: 0.75rem;
		width: calc(2 * var(--thumb-half));
		height: calc(2 * var(--thumb-half));
		border-radius: 50%;
		background: var(--bs-emphasis-color);
		opacity: 0.25;
		transform: translate(-50%, -50%);
		pointer-events: none;
	}

	.ticks {
		position: relative;
		height: 1.2rem;
		margin-top: 0.1rem;
	}

	.tick {
		position: absolute;
		top: 0;
		transform: translateX(-50%);
		white-space: nowrap;
		color: var(--bs-body-color);
		font-size: 0.78rem;
		opacity: 0.55;
	}

	.tick.on {
		opacity: 1;
		font-weight: 600;
	}
</style>
