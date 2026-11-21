<script lang="ts">
	import type { Snippet } from 'svelte';
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
		/** Longer explanation (any markup), shown in a modal opened from the info button. */
		help?: Snippet;
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

	let helpDialog = $state<HTMLDialogElement>();
	const helpTitleId = $derived(`${store.elementId(path)}-help-title`);
</script>

<div class="setting-row" id={store.elementId(path)} class:changed class:tweaked class:compact hidden={!store.matches(info)}>
	<div class="left">
		<div class="label-col">
			{#if help}
				<button type="button" class="btn btn-link btn-sm p-0 help" aria-label="Help: {label}" onclick={() => helpDialog?.showModal()}>
					<i class="bi bi-info-circle"></i>
				</button>
			{/if}
			<label
				class="label"
				class:clickable={labelFor && !store.locked}
				class:locked={labelFor && store.locked}
				for={labelFor}
				data-lockable={labelFor ? '' : undefined}
			>{label}</label>
			{#if badge && !compact}<span class="badge-tag">{badge}</span>{/if}
			{#if changed && !store.locked}
				<button
					type="button"
					class="btn btn-link btn-sm p-0 reset"
					title={`Reset to ${format(getAt(store.applied, path) as Value)}`}
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
		<div data-lockable>{@render control()}</div>
		{#if nested}
			<div class="nested">{@render nested()}</div>
		{/if}
	</div>

	{#if help}
		<!-- Escape, Close or a click on the backdrop closes it. -->
		<dialog
			bind:this={helpDialog}
			class="help-dialog"
			aria-labelledby={helpTitleId}
			onclick={(e) => {
				if (e.target === helpDialog) helpDialog.close();
			}}
		>
			<div class="p-4">
				<h2 id={helpTitleId} class="h5 fw-bold mb-3">{label}</h2>
				<div class="help-body">{@render help()}</div>
				<div class="d-flex justify-content-end mt-3">
					<button type="button" class="btn btn-outline-secondary btn-sm" onclick={() => helpDialog?.close()}>Close</button>
				</div>
			</div>
		</dialog>
	{/if}
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
		background-color: rgba(var(--bs-success-rgb), 0.25);
	}

	/* Cyan rather than green, so it reads as distinct from a change rather than a fainter one. */
	.setting-row.tweaked {
		background-color: rgba(var(--bs-info-rgb), 0.15);
	}

	/* Hover highlight, as an image layer so it stacks on top of any change/tweak shading. */
	.setting-row:hover {
		background-image: linear-gradient(rgba(var(--bs-emphasis-color-rgb), 0.05), rgba(var(--bs-emphasis-color-rgb), 0.05));
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
		align-items: start;
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

	.label.locked {
		cursor: not-allowed;
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
		line-height: 1;
		color: inherit;
	}

	.help:hover,
	.help:focus-visible {
		opacity: 0.9;
	}

	.help-dialog {
		width: min(32rem, calc(100vw - 2rem));
		padding: 0;
		border: 1px solid var(--bs-border-color);
		border-radius: 0.6rem;
		background: var(--bs-body-bg);
		color: var(--bs-body-color);
	}

	.help-dialog::backdrop {
		background: rgb(0 0 0 / 0.55);
	}

	.help-body :global(> :last-child) {
		margin-bottom: 0;
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
