<script lang="ts">
	import type { ChoiceOption } from '../types';

	let {
		options,
		value,
		label,
		disabled = false,
		onchange
	}: {
		options: ChoiceOption[];
		value: string;
		label: string;
		disabled?: boolean;
		onchange: (v: string) => void;
	} = $props();
</script>

<div class="cards" role="radiogroup" aria-label={label}>
	{#each options as o (o.value)}
		<button
			type="button"
			role="radio"
			aria-checked={value === o.value}
			{disabled}
			class="opt-card"
			class:selected={value === o.value}
			onclick={() => onchange(o.value)}
		>
			<span class="title">
				{#if o.swatch}<span class="swatch" style:background={o.swatch}></span>{/if}
				{o.label}
			</span>
			{#if o.desc}<span class="desc">{o.desc}</span>{/if}
		</button>
	{/each}
</div>

<style>
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
		gap: 0.5rem;
	}

	.opt-card {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		text-align: left;
		padding: 0.5rem 0.7rem;
		border: 1px solid var(--bs-border-color);
		border-radius: 0.5rem;
		background: var(--bs-body-bg);
		color: var(--bs-body-color);
		cursor: pointer;
		transition: border-color 0.12s, background 0.12s;
	}

	.opt-card:hover:not(:disabled) {
		border-color: var(--bs-primary);
	}

	.opt-card.selected {
		border-color: var(--bs-primary);
		background: color-mix(in srgb, var(--bs-primary) 18%, var(--bs-body-bg));
		box-shadow: 0 0 0 1px var(--bs-primary);
	}

	/* Locked: the chosen card stays highlighted, the rest fade back. */
	.opt-card:disabled {
		cursor: default;
	}

	.opt-card:disabled:not(.selected) {
		opacity: 0.45;
	}

	.title {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		font-weight: 600;
	}

	.swatch {
		width: 0.75rem;
		height: 0.75rem;
		border-radius: 50%;
		box-shadow: 0 0 0 1px rgb(255 255 255 / 0.35);
		flex: none;
	}

	.desc {
		font-size: 0.78rem;
		opacity: 0.7;
		line-height: 1.25;
	}
</style>
