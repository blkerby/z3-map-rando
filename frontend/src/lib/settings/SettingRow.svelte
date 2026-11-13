<script lang="ts">
	import type { Snippet } from 'svelte';
	import { tooltip } from '$lib/tooltip';
	import { getCardId, getCompact, getStore } from './context';
	import { getAt, type Path, type Value } from './tree';

	// The frame shared by every setting component: label, help, reset, shading, and registration with the
	// store (which is how the summary panel and search know about the setting).
	let {
		path,
		label,
		help,
		notes,
		badge,
		format,
		labelFor,
		control,
		nested
	}: {
		path: Path;
		label: string;
		/** Longer explanation, shown in a tooltip. */
		help?: string;
		/** Short descriptor shown in small text under the label. Searchable. */
		notes?: string;
		/** Small tag next to the label, e.g. a difficulty. */
		badge?: string;
		format: (value: Value) => string;
		/** Id of a switch the label toggles. */
		labelFor?: string;
		control: Snippet;
		/** Rows shown indented underneath (e.g. options of a toggle). */
		nested?: Snippet;
	} = $props();

	const store = getStore();
	const cardId = getCardId();
	const compact = getCompact();

	const info = $derived({ path, label, notes, cardId, format });
	$effect(() => store.register(info));

	const changed = $derived(store.isChanged(path));
	/** One of the selected preset's tweaks (shaded cyan, unlike the green of an unsaved change). */
	const tweaked = $derived(store.isTweaked(path));
</script>

<div class="setting-row" id={store.elementId(path)} class:changed class:tweaked class:compact hidden={!store.matches(info)}>
	<div class="left">
		<div class="label-col">
			{#if help}
				<i class="bi bi-info-circle help" use:tooltip={help} aria-label={help}></i>
			{/if}
			<label class="label" class:clickable={labelFor && !store.locked} for={labelFor}>{label}</label>
			{#if badge && !compact}<span class="badge-tag">{badge}</span>{/if}
			{#if changed && !store.locked}
				<button
					type="button"
					class="btn btn-link btn-sm p-0 reset"
					use:tooltip={`Reset to ${format(getAt(store.applied, path) as Value)}`}
					aria-label="Reset {label}"
					onclick={() => store.reset(path)}
				>
					<i class="bi bi-arrow-counterclockwise"></i>
				</button>
			{/if}
		</div>
		<!-- In compact rows the name needs the room, so the tag drops to a small second line. -->
		{#if badge && compact}<div class="badge-sub">{badge}</div>{/if}
		{#if notes}<div class="notes">{notes}</div>{/if}
	</div>

	<div class="control-col">
		{@render control()}
		{#if nested}
			<div class="nested">{@render nested()}</div>
		{/if}
	</div>
</div>

<style>
	/* Label on top, control underneath, full width. */
	.setting-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 0.4rem;
		padding: 0.85rem 0.5rem;
		/* Bleed out by the padding so the tint doesn't hug the text and the layout doesn't shift. */
		margin-inline: -0.5rem;
		border-radius: var(--bs-border-radius);
		position: relative;
	}

	.setting-row[hidden] {
		display: none;
	}

	.setting-row.changed {
		background: rgba(var(--bs-success-rgb), 0.25);
	}

	/* Cyan rather than green, so it reads as distinct from a change rather than a fainter one. */
	.setting-row.tweaked {
		background: rgba(var(--bs-info-rgb), 0.15);
	}

	/* Hairline between full rows. Drawn separately, since a border would curve with the row's rounded corners. */
	:global(.setting-row:not(.compact) + .setting-row:not(.compact)::before) {
		content: '';
		position: absolute;
		inset: 0 0 auto;
		border-top: 1px solid var(--bs-border-color-translucent);
	}

	/* Compact: label on the left, control on the right, for the grid and compact layouts. */
	.setting-row.compact {
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.5rem;
		padding-block: 0.3rem;
	}

	.compact .label-col {
		min-width: 0;
	}

	/* Never cut a name off: long ones wrap (or, for one unbroken word, break) inside their cell. */
	.compact .label {
		overflow-wrap: anywhere;
	}

	.label-col {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.label {
		font-weight: 500;
		cursor: default;
	}

	.label.clickable {
		cursor: pointer;
	}

	.left {
		min-width: 0;
	}

	.notes {
		font-size: 0.78rem;
		line-height: 1.25;
		opacity: 0.7;
	}

	.badge-sub {
		font-size: 0.7rem;
		line-height: 1.1;
		opacity: 0.65;
	}

	.badge-tag {
		white-space: nowrap;
		padding: 0 0.45rem;
		border: 1px solid var(--bs-border-color);
		border-radius: 999px;
		font-size: 0.7rem;
		opacity: 0.7;
	}

	.help {
		opacity: 0.5;
		font-size: 0.85rem;
		cursor: help;
	}

	.reset {
		margin-left: 0.25rem;
		text-decoration: none;
	}

	.control-col {
		min-width: 0;
	}

	.nested:not(:empty) {
		margin-top: 0.4rem;
		padding-left: 0.9rem;
		border-left: 2px solid var(--bs-border-color);
	}
</style>
