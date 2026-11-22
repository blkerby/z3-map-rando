<script lang="ts">
	import type { ChoiceOption } from '../types';

	// Color picker drawn as pixel hearts (the life-meter heart). The white border keeps every color,
	// yellow included, readable against the theme. Every heart has a dark blue outline; the selected
	// one's outline turns gold (--bs-primary).
	let {
		options,
		value,
		label,
		disabled = false,
		columns = 10,
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

	// Each art pixel covers a whole number of device pixels (about 3 CSS px worth), so crispEdges
	// can't round some pixels larger than others at fractional scales (browser zoom, desktop
	// scaling). The scale is re-read whenever it changes.
	let dpr = $state(1);
	$effect(() => {
		let mq: MediaQueryList;
		const update = () => {
			const r = window.devicePixelRatio;
			dpr = r;
			mq = matchMedia(`(resolution: ${r}dppx)`);
			mq.addEventListener('change', update, { once: true });
		};
		update();
		return () => mq.removeEventListener('change', update);
	});
	const heartSize = $derived(((SIZE + PAD * 2) * Math.max(1, Math.round(3 * dpr))) / dpr);
</script>

<div
	class="swatches"
	class:locked={disabled}
	role="radiogroup"
	aria-label={label}
	style:--cols={columns}
	style:--heart="{heartSize}px"
>
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
			<svg
				viewBox="{-PAD} {-PAD} {SIZE + PAD * 2} {SIZE + PAD * 2}"
				aria-hidden="true"
				shape-rendering="crispEdges"
			>
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

<style>
	/* Wraps instead of scrolling (a scroll container would clip the outline and hover growth). */
	.swatches {
		display: grid;
		grid-template-columns: repeat(var(--cols), var(--heart));
		gap: 0.25rem;
		max-width: 100%;
		/* Centers the first row of hearts on the 1.5rem label line beside it. */
		margin-top: calc((1.5rem - var(--heart)) / 2);
		padding-bottom: 0.35rem;
	}

	.heart {
		display: block;
		width: var(--heart);
		height: var(--heart);
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
		cursor: not-allowed;
	}

	.locked .heart:not(.selected) {
		opacity: 0.35;
	}
</style>
