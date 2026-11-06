<script lang="ts">
	import type { ChoiceOption } from '../types';

	// Color picker drawn as pixel hearts (the life-meter heart). The white border keeps every color,
	// yellow included, readable against the theme. Every heart has a dark blue outline; the selected
	// one's outline turns gold (--bs-primary); its name shows underneath.
	let {
		options,
		value,
		label,
		disabled = false,
		columns = 12,
		onchange
	}: {
		options: ChoiceOption[];
		value: string;
		label: string;
		disabled?: boolean;
		/** Hearts per row. */
		columns?: number;
		onchange: (v: string) => void;
	} = $props();

	const selectedLabel = $derived(options.find((o) => o.value === value)?.label ?? '');

	// The heart, one character per pixel: 0 transparent, 1 white, 2 the chosen color.
	const ART = ['0110110', '1221221', '1222221', '1222221', '0122210', '0012100', '0001000'];
	const SIZE = ART.length;
	const PAD = 1; // room around the heart for the selection outline

	const pixels: { x: number; y: number; c: '1' | '2' }[] = [];
	const halo: { x: number; y: number }[] = [];
	const filled = (x: number, y: number) => ART[y]?.[x] !== undefined && ART[y][x] !== '0';

	for (let y = -PAD; y < SIZE + PAD; y++) {
		for (let x = -PAD; x < SIZE + PAD; x++) {
			if (filled(x, y)) pixels.push({ x, y, c: ART[y][x] as '1' | '2' });
			else if ([-1, 0, 1].some((dy) => [-1, 0, 1].some((dx) => filled(x + dx, y + dy)))) {
				halo.push({ x, y });
			}
		}
	}
</script>

<div class="swatches" class:locked={disabled} role="radiogroup" aria-label={label} style:--cols={columns}>
	{#each options as o (o.value)}
		<button
			type="button"
			role="radio"
			aria-checked={value === o.value}
			aria-label={o.label}
			title={o.label}
			{disabled}
			class="heart"
			class:selected={value === o.value}
			onclick={() => onchange(o.value)}
		>
			<svg viewBox="{-PAD} {-PAD} {SIZE + PAD * 2} {SIZE + PAD * 2}" aria-hidden="true" shape-rendering="crispEdges">
				{#each halo as h (`${h.x},${h.y}`)}
					<rect x={h.x} y={h.y} width="1" height="1" class="halo" />
				{/each}
				{#each pixels as px (`${px.x},${px.y}`)}
					<rect x={px.x} y={px.y} width="1" height="1" fill={px.c === '2' ? o.swatch : '#fff'} />
				{/each}
			</svg>
		</button>
	{/each}
</div>
<div class="name small text-body-secondary">{selectedLabel}</div>

<style>
	/* Wraps instead of scrolling (a scroll container would clip the outline and hover growth). */
	.swatches {
		display: grid;
		grid-template-columns: repeat(var(--cols), 2rem);
		gap: 0.3rem;
		max-width: 100%;
		padding: 0.35rem 0;
	}

	@media (max-width: 36rem) {
		.swatches {
			grid-template-columns: repeat(6, 2rem);
		}
	}

	.heart {
		display: block;
		width: 2rem;
		height: 2rem;
		padding: 0;
		border: 0;
		background: none;
		cursor: pointer;
		transition: transform 0.1s;
	}

	.heart svg {
		display: block;
		width: 100%;
		height: 100%;
		overflow: visible;
	}

	/* Every heart has a dark blue outline; the chosen one's turns gold. */
	.halo {
		fill: #000050;
	}

	.selected .halo {
		fill: var(--bs-primary);
	}

	.heart:hover:not(:disabled) {
		transform: scale(1.15);
	}

	.heart:focus-visible {
		outline: 2px solid var(--bs-primary);
		outline-offset: 2px;
	}

	/* Locked: the chosen color stays outlined, the rest fade back. */
	.locked .heart {
		cursor: default;
	}

	.locked .heart:not(.selected) {
		opacity: 0.35;
	}

	.name {
		margin-top: 0.1rem;
	}
</style>