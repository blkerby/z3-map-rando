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

<div class="btn-group flex-wrap" class:locked={disabled} role="radiogroup" aria-label={label}>
	{#each options as o (o.value)}
		<button
			type="button"
			role="radio"
			aria-checked={value === o.value}
			{disabled}
			class="btn btn-sm"
			class:btn-primary={value === o.value}
			class:btn-outline-primary={value !== o.value}
			onclick={() => onchange(o.value)}
		>
			{#if o.swatch}<span class="swatch" style:background={o.swatch}></span>{/if}
			{o.label}
		</button>
	{/each}
</div>

<style>
	.swatch {
		display: inline-block;
		width: 0.65rem;
		height: 0.65rem;
		margin-right: 0.35rem;
		border-radius: 50%;
		vertical-align: -0.05rem;
		box-shadow: 0 0 0 1px rgb(255 255 255 / 0.4);
	}

	/* Locked: the chosen option stays clearly highlighted, the rest fade back. */
	.locked .btn {
		cursor: default;
	}

	.locked .btn:disabled.btn-primary {
		opacity: 0.9;
	}

	.locked .btn:disabled.btn-outline-primary {
		opacity: 0.4;
	}
</style>
