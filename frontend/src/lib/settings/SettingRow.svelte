<script lang="ts">
	import type { Snippet } from 'svelte';
	import { getCardId, getCompact, getStore } from './context';
	import { narrow } from './narrow';
	import { getAt, type Path, type Value } from './tree';

	// The frame shared by every setting component: label, help, reset, shading, and registration with the
	// store (which is how the summary panel and search know about the setting).
	let {
		path,
		label,
		help,
		notes,
		badge,
		icon,
		format,
		labelFor,
		control,
		nested,
		wrap = false
	}: {
		path: Path;
		label: string;
		/** Longer explanation (any markup), shown in a modal opened from the info button. */
		help?: Snippet;
		/** Short descriptor shown in small text under the label. Searchable. */
		notes?: string;
		/** Small tag next to the label, e.g. a difficulty. */
		badge?: string;
		/** URL of a 16×16 pixel-art icon shown before the label; null holds its place empty, keeping labels aligned. */
		icon?: string | null;
		format: (value: Value) => string;
		/** Id of a switch the label toggles. */
		labelFor?: string;
		control: Snippet;
		/** Rows shown indented underneath (e.g. options of a toggle). */
		nested?: Snippet;
		/** In a compact row, put the control beneath the label when they don't fit side by side. */
		wrap?: boolean;
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

<!-- A switch's row is one big target for it: a click anywhere that isn't on another control (or on
     the label or switch, which handle themselves) toggles it. Keyboard users have the switch itself. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
	class="setting-row"
	id={store.elementId(path)}
	class:changed
	class:tweaked
	class:compact
	class:wrap
	class:clickable={labelFor && !store.locked}
	class:locked={labelFor && store.locked}
	hidden={!store.matches(info)}
	data-lockable={labelFor ? '' : undefined}
	onclick={(e) => {
		if (labelFor && (e.target as Element).closest('[data-lockable]') === e.currentTarget) document.getElementById(labelFor)?.click();
	}}
>
	<div class="left">
		<div class="label-col">
			{#if help}
				<button type="button" class="btn btn-link btn-sm p-0 help" data-lockable="false" aria-label="Help: {label}" onclick={() => helpDialog?.showModal()}>
					<i class="bi bi-info-circle"></i>
				</button>
			{/if}
			{#if icon !== undefined}
				<span class="icon">{#if icon !== null}<img src={icon} alt="" />{/if}</span>
			{/if}
			<label class="label" for={labelFor} data-lockable={labelFor ? '' : undefined}>{label}</label>
			{#if badge && !compact}<span class="badge-tag">{badge}</span>{/if}
			<!-- Left out on narrow screens for room: the setting's own control can put it back. -->
			{#if changed && !store.locked && !narrow.current}
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
		<div data-lockable class:switch-wrap={labelFor}>{@render control()}</div>
		{#if nested}
			<div class="nested" data-lockable="false">{@render nested()}</div>
		{/if}
	</div>

	{#if help}
		<!-- Escape, Close or a click on the backdrop closes it. `autofocus` makes `showModal()` focus the dialog
		     itself rather than its first button. -->
		<!-- svelte-ignore a11y_autofocus -->
		<dialog
			bind:this={helpDialog}
			autofocus
			class="help-dialog"
			data-lockable="false"
			aria-labelledby={helpTitleId}
			onclick={(e) => {
				if (e.target === helpDialog) helpDialog.close();
			}}
		>
			<div class="d-flex align-items-center px-4 py-3 border-bottom">
				<h2 id={helpTitleId} class="h5 fw-bold mb-0 me-auto">{label}</h2>
				<button type="button" class="btn-close" aria-label="Close" onclick={() => helpDialog?.close()}></button>
			</div>
			<div class="p-4">
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
		--pad-y: 0.85rem;
		padding: var(--pad-y) 0.5rem;
		/* Bleed out by the padding so the tint doesn't hug the text and the layout doesn't shift. */
		margin-inline: -0.5rem;
		border-radius: var(--bs-border-radius);
		position: relative;
		cursor: default;
	}

	.setting-row.clickable {
		cursor: pointer;
	}

	.setting-row.locked {
		cursor: not-allowed;
	}

	.setting-row[hidden] {
		display: none;
	}

	.setting-row.changed {
		background-color: rgba(var(--bs-success-rgb), 0.15);
	}

	/* Cyan rather than green, so it reads as distinct from a change rather than a fainter one. */
	.setting-row.tweaked {
		background-color: rgba(var(--bs-info-rgb), 0.15);
	}

	/* Hover highlight for rows that are one big click target (switch rows), as an image layer so it
	   stacks on top of any change/tweak shading. */
	.setting-row[data-lockable]:hover {
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
		--pad-y: 0.45rem;
	}

	/* The control sits beside the label if both fit at full width, else beneath it, shrinking to fit. */
	.setting-row.compact.wrap {
		display: flex;
		flex-wrap: wrap;
	}

	.wrap .left {
		flex: 1 0 auto;
	}

	/* Center Bootstrap's small controls (1.5em at 0.875rem, plus 0.5rem padding and borders) on the 1.5rem
	   label line. They stay within the row's padding, so rows keep their spacing. The button group is
	   inline, so it is also taken off the text baseline. */
	.compact .control-col :global(:is(.form-control-sm, .form-select-sm, .btn-group)) {
		margin-block: calc(-0.15625rem - var(--bs-border-width));
		vertical-align: top;
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
	}

	/* 2.25 times the art's size. The negative margin keeps the row's height set by its label, so the icon
	   extends into the row's padding. */
	.icon {
		flex: none;
		width: 36px;
		height: 36px;
		margin-block: -6px;
	}

	.icon img {
		display: block;
		width: 100%;
		height: 100%;
		image-rendering: pixelated;
	}

	/* The containing block for the help button's click area. */
	.left {
		position: relative;
		min-width: 0;
	}

	/* Full height, so the help button's click area can span the row. */
	.compact .left {
		align-self: stretch;
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

	/* Borderless, so the button is just its 1em icon (which the click area's width relies on). */
	.help {
		border: 0;
		opacity: 0.5;
		font-size: 0.85rem;
		line-height: 1;
		color: inherit;
	}

	/* The help button's click area: the whole strip left of the label, from the row's left edge to the label,
	   and from the row's top to its bottom (in full rows, to halfway into the gap above the control). */
	.help::before {
		content: '';
		position: absolute;
		top: calc(-1 * var(--pad-y));
		bottom: -0.2rem;
		left: -0.5rem;
		width: calc(0.5rem + 1em + 0.4rem);
	}

	.compact .help::before {
		bottom: calc(-1 * var(--pad-y));
	}

	.help:hover,
	.help:focus-visible {
		opacity: 0.9;
	}

	.help-dialog {
		cursor: auto;
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

	/* Only as wide as the switch, so the space beside it is part of the row's click target. */
	.switch-wrap {
		width: fit-content;
	}

	.nested {
		cursor: default;
	}

	.nested:not(:empty) {
		margin-top: 0.4rem;
		padding-left: 0.9rem;
		border-left: 2px solid var(--bs-border-color);
	}
</style>
