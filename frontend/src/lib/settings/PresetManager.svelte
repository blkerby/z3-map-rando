<script lang="ts">
	import type { Snippet } from 'svelte';
	import { tooltip } from '$lib/tooltip';
	import type { SettingsStore } from './store.svelte';
	import type { Section } from './types';
	import {
		MAX_IMPORT_BYTES,
		MAX_NAME,
		MAX_USER_PRESETS,
		buildExportFile,
		cleanName,
		encodePresetCode,
		parseImport,
		sameName,
		uniqueName,
		type PresetKind
	} from './presetIO';

	let {
		store,
		section: card,
		kind,
		selectId = 'preset',
		children
	}: {
		store: SettingsStore;
		/** Manage this card's own presets instead of the presets for all the settings. */
		section?: Section;
		kind: PresetKind;
		selectId?: string;
		/** Extra controls shown at the end of the row (e.g. a search box). */
		children?: Snippet;
	} = $props();

	type Mode = 'save' | 'rename' | 'delete' | 'import' | null;

	let mode = $state<Mode>(null);
	let menuOpen = $state(false);
	let nameDraft = $state('');
	let importText = $state('');
	let formError = $state<string | null>(null);
	let overwriteConfirm = $state(false);
	/** Shown only for what isn't visible on screen: a failure, or an import that didn't go as asked. */
	let notice = $state<{ type: 'info' | 'danger'; text: string } | null>(null);

	let menuEl = $state<HTMLElement>();
	let dialog = $state<HTMLDialogElement>();
	let nameInput = $state<HTMLInputElement>();
	let importArea = $state<HTMLTextAreaElement>();
	let fileInput = $state<HTMLInputElement>();

	/** The section whose presets these are: the card's, or the root (all the settings). */
	const section = $derived(card ?? store.root);
	const list = $derived(store.listFor(section));
	/** The preset the section takes (picked by hand, or chosen by the presets around it). */
	const current = $derived(store.selectedPreset(section));
	/** The same, if it is one of the player's own. */
	const selected = $derived(list.isUser(current.id) ? current : undefined);
	/** A card that isn't picked by hand, where the presets around it change something: "Hard + 2 tweaks". */
	const tweaks = $derived(card ? store.tweakCount(card) : 0);
	/** The card's preset was picked by hand: an unsaved change. */
	const picked = $derived(store.isSelectionChanged(section));
	const changed = $derived(store.sectionChanged(section));
	/** Unsaved changes inside the section, besides its own pick (which the dropdown already names). */
	const changeCount = $derived(store.countChanges(section) - (picked ? 1 : 0));
	/**
	 * What the section takes, with its unsaved changes and the built-in preset behind one of the player's own:
	 * "My settings (based on Default) + 1 unsaved change".
	 */
	const label = $derived(
		(card && !picked ? store.inheritedLabel(card) : current.name) +
			(selected ? ` (based on ${list.builtInBehind(selected).name})` : '') +
			(changeCount ? ` + ${changeCount} unsaved change${changeCount === 1 ? '' : 's'}` : '')
	);
	/** Value of the hidden entry that shows `label` while there are unsaved changes (not a preset id). */
	const CURRENT = '*current';
	/**
	 * What the dropdown shows: the entry for the unsaved changes, else the preset picked by hand, else ""
	 * for the "+ tweaks" entry, else the preset chosen for it.
	 */
	const shown = $derived(changeCount ? CURRENT : picked || !tweaks ? current.id : '');
	const noun = $derived(card ? `${card.title.toLowerCase()} settings` : kind === 'settings' ? 'settings' : 'customization');

	// ---- Menu ---------------------------------------------------------------------
	function onWindowClick(e: MouseEvent) {
		if (menuOpen && menuEl && !menuEl.contains(e.target as Node)) menuOpen = false;
	}

	function onWindowKey(e: KeyboardEvent) {
		if (e.key !== 'Escape') return;
		if (menuOpen) menuOpen = false;
	}

	function closeForm() {
		mode = null;
		formError = null;
		overwriteConfirm = false;
	}

	function openMode(next: Exclude<Mode, null>) {
		menuOpen = false;
		notice = null;
		formError = null;
		overwriteConfirm = false;
		mode = next;
		if (next === 'save') nameDraft = '';
		if (next === 'rename') nameDraft = selected?.name ?? '';
		if (next === 'import') importText = '';
	}

	// Keep the native <dialog> in step with `mode`, focusing the field to type in.
	$effect(() => {
		if (!dialog) return;
		if (mode && !dialog.open) dialog.showModal();
		else if (!mode && dialog.open) dialog.close();
		if ((mode === 'save' || mode === 'rename') && nameInput) nameInput.focus();
		else if (mode === 'import' && importArea) importArea.focus();
	});

	// ---- Save / rename / update / delete -----------------------------------------
	function submitName() {
		const name = cleanName(nameDraft);
		if (!name) {
			formError = 'Give the preset a name.';
			return;
		}
		if (list.builtIn.some((p) => sameName(p.name, name))) {
			formError = `“${name}” is a built-in preset name. Please choose another.`;
			return;
		}
		const renaming = mode === 'rename';
		const clash = list.userPresets.find(
			(p) => sameName(p.name, name) && !(renaming && p.id === selected?.id)
		);

		if (renaming) {
			if (!selected) return;
			if (clash) {
				formError = 'You already have a preset with that name.';
				return;
			}
			list.rename(selected.id, name);
			closeForm();
			return;
		}

		if (clash) {
			// Two-step: the first press warns, the second replaces.
			if (!overwriteConfirm) {
				overwriteConfirm = true;
				formError = `You already have a preset called “${clash.name}”. Press Save again to replace it.`;
				return;
			}
			list.update(clash.id, store.capture(section));
			store.adoptSaved(section, clash.id);
			closeForm();
			return;
		}

		if (list.userPresets.length >= MAX_USER_PRESETS) {
			formError = `You can keep up to ${MAX_USER_PRESETS} presets. Delete some first.`;
			return;
		}
		const preset = list.add(name, store.capture(section));
		store.adoptSaved(section, preset.id);
		closeForm();
	}

	function updateSelected() {
		menuOpen = false;
		if (!selected) return;
		list.update(selected.id, store.capture(section));
		store.adoptSaved(section, selected.id);
	}

	function confirmDelete() {
		if (!selected) return;
		store.deletePreset(section, selected.id);
		closeForm();
	}

	// ---- Share: copy code / export file / import ---------------------------------
	async function copyCode() {
		menuOpen = false;
		const name = changed ? 'Shared settings' : current.name;
		try {
			await navigator.clipboard.writeText(encodePresetCode(kind, name, store.capture(section)));
		} catch {
			notice = { type: 'danger', text: "Couldn't copy to the clipboard." };
		}
	}

	function exportAll() {
		menuOpen = false;
		const blob = new Blob([buildExportFile(kind, list.userPresets)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `alttp-maprando-${kind}-presets.json`;
		document.body.append(a);
		a.click();
		a.remove();
		setTimeout(() => URL.revokeObjectURL(url), 1000);
	}

	function runImport(text: string) {
		const result = parseImport(text, kind, (raw) => store.sanitize(raw));
		if (!result.ok) {
			formError = result.error;
			return;
		}
		const room = MAX_USER_PRESETS - list.userPresets.length;
		if (room <= 0) {
			formError = `You already have ${MAX_USER_PRESETS} presets. Delete some first.`;
			return;
		}

		const taken = list.all.map((p) => p.name);
		const added = [];
		let renamed = 0;
		for (const item of result.presets.slice(0, room)) {
			const name = uniqueName(item.name, taken);
			if (name !== item.name) renamed++;
			taken.push(name);
			added.push(list.add(name, { base: item.base, sections: item.sections, values: item.values }));
		}
		const skipped = result.presets.length - added.length;

		// A single preset is applied straight away so you can see what you imported.
		if (added.length === 1 && !store.locked) store.applyPreset(added[0].id, section);

		if (renamed || skipped) {
			let text2 = `Imported ${added.length} preset${added.length === 1 ? '' : 's'}`;
			if (renamed) text2 += ` (${renamed} renamed to avoid duplicates)`;
			if (skipped) text2 += `; ${skipped} skipped because the list is full`;
			notice = { type: 'info', text: text2 + '.' };
		}
		closeForm();
	}

	async function onFile(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		formError = null;
		if (file.size > MAX_IMPORT_BYTES) {
			formError = 'That file is too large to be a preset file.';
			return;
		}
		runImport(await file.text());
	}
</script>

<svelte:window onclick={onWindowClick} onkeydown={onWindowKey} />

<div class="preset-manager">
	<div class="controls">
		<select
			id={selectId}
			aria-label="Preset"
			class="form-select form-select-sm w-auto preset-select"
			class:changed
			class:tweaked={!changed && store.isSelectionTweaked(section)}
			use:tooltip={picked && card ? `Picked by hand. Reset goes back to “${store.inheritedLabel(card)}”.` : undefined}
			disabled={store.locked}
			value={shown}
			onchange={(e) => store.applyPreset(e.currentTarget.value, section)}
		>
			{#if changeCount}
				<option value={CURRENT} hidden disabled>{label}</option>
			{/if}
			{#if card && tweaks}
				<option value="">{store.inheritedLabel(card)}</option>
			{/if}
			<optgroup label="Built-in">
				{#each list.builtIn as p (p.id)}
					<option value={p.id}>{p.name}</option>
				{/each}
			</optgroup>
			{#if list.userPresets.length}
				<optgroup label="My presets">
					{#each list.userPresets as p (p.id)}
						<option value={p.id}>{p.name} (based on {list.builtInBehind(p).name})</option>
					{/each}
				</optgroup>
			{/if}
		</select>

		{#if selected && store.canUpdate(section)}
			<button
				type="button"
				class="btn btn-link btn-sm p-0 icon-btn"
				use:tooltip={`Update “${selected.name}” with your current ${noun}.`}
				aria-label="Update “{selected.name}”"
				onclick={updateSelected}
			>
				<i class="bi bi-floppy"></i>
			</button>
		{:else if !selected && changeCount}
			<button
				type="button"
				class="btn btn-link btn-sm p-0 icon-btn"
				use:tooltip={`Save your current ${noun} as a new preset.`}
				aria-label="Save as new preset"
				onclick={() => openMode('save')}
			>
				<i class="bi bi-floppy"></i>
			</button>
		{/if}

		<div class="menu-wrap" bind:this={menuEl}>
			<button
				type="button"
				class="btn btn-link btn-sm p-0 icon-btn"
				aria-haspopup="menu"
				aria-expanded={menuOpen}
				aria-label="Manage presets"
				use:tooltip={'Manage presets'}
				onclick={() => (menuOpen = !menuOpen)}
			>
				<i class="bi bi-three-dots"></i>
			</button>
			{#if menuOpen}
				<div class="menu" role="menu">
					<button type="button" role="menuitem" class="menu-item" onclick={() => openMode('save')}>
						<i class="bi bi-plus-circle"></i> Save current as new preset…
					</button>
					<button
						type="button"
						role="menuitem"
						class="menu-item"
						disabled={!selected}
						onclick={() => openMode('rename')}
					>
						<i class="bi bi-pencil"></i> Rename…
					</button>
					<button
						type="button"
						role="menuitem"
						class="menu-item"
						disabled={!selected || store.locked}
						onclick={() => {
							menuOpen = false;
							store.detachPreset(section);
						}}
					>
						<i class="bi bi-link-45deg"></i>
						{selected ? `Detach from “${selected.name}”` : 'Detach from preset'}
					</button>
					<button
						type="button"
						role="menuitem"
						class="menu-item"
						disabled={!selected}
						onclick={() => openMode('delete')}
					>
						<i class="bi bi-trash"></i> Delete…
					</button>
					<hr class="menu-sep" />
					<button type="button" role="menuitem" class="menu-item" onclick={copyCode}>
						<i class="bi bi-clipboard"></i> Copy {noun} as code
					</button>
					<button
						type="button"
						role="menuitem"
						class="menu-item"
						disabled={!list.userPresets.length}
						onclick={exportAll}
					>
						<i class="bi bi-box-arrow-up"></i> Export my presets…
					</button>
					<button type="button" role="menuitem" class="menu-item" onclick={() => openMode('import')}>
						<i class="bi bi-box-arrow-in-down"></i> Import…
					</button>
				</div>
			{/if}
		</div>

		<!-- A card's own Reset sits in its header (see the generate page), so this is only for the top-level preset. -->
		{#if !card && changed}
			<button
				type="button"
				class="btn btn-link btn-sm p-0 icon-btn"
				disabled={store.locked}
				use:tooltip={`Put all the ${noun} back to ${current.name}.`}
				aria-label="Reset all the {noun}"
				onclick={() => store.resetAll()}
			>
				<i class="bi bi-arrow-counterclockwise"></i>
			</button>
		{/if}

		{@render children?.()}
	</div>

	{#if !card && store.storageFailed}
		<div class="alert alert-danger small py-1 px-2 mt-2 mb-0" role="status">
			Your presets couldn't be saved in this browser (storage may be full or blocked). Export them to keep a backup.
		</div>
	{/if}
	{#if notice}
		<div class="alert alert-{notice.type} small py-1 px-2 mt-2 mb-0 d-flex align-items-center" role="status">
			<span class="me-auto">{notice.text}</span>
			<button type="button" class="btn-close btn-sm" aria-label="Dismiss" onclick={() => (notice = null)}
			></button>
		</div>
	{/if}

	<!-- A modal, so the forms don't push the header around. Escape, Cancel or a click on the backdrop closes it. -->
	<dialog
		bind:this={dialog}
		class="preset-dialog"
		aria-labelledby="{selectId}-dialog-title"
		oncancel={(e) => {
			e.preventDefault();
			closeForm();
		}}
		onclick={(e) => {
			if (e.target === dialog) closeForm();
		}}
	>
		<div class="p-4">
			<h2 id="{selectId}-dialog-title" class="h5 fw-bold mb-3">
				{mode === 'save' ? 'Save preset' : mode === 'rename' ? 'Rename preset' : mode === 'delete' ? 'Delete preset' : 'Import presets'}
			</h2>
			{#if mode === 'save' || mode === 'rename'}
				<form
					onsubmit={(e) => {
						e.preventDefault();
						submitName();
					}}
				>
					<label class="form-label small mb-1" for="{selectId}-name">
						{mode === 'save' ? `Save your current ${noun} as…` : `New name for “${selected?.name}”`}
					</label>
					<input
						id="{selectId}-name"
						class="form-control form-control-sm"
						type="text"
						autocomplete="off"
						maxlength={MAX_NAME}
						placeholder="Preset name"
						bind:this={nameInput}
						bind:value={nameDraft}
						oninput={() => {
							overwriteConfirm = false;
							formError = null;
						}}
					/>
					{#if formError}<div class="form-text text-danger">{formError}</div>{/if}
					<div class="d-flex justify-content-end gap-2 mt-3">
						<button type="button" class="btn btn-outline-secondary btn-sm" onclick={closeForm}>Cancel</button>
						<button type="submit" class="btn btn-primary btn-sm">
							{mode === 'save' ? 'Save' : 'Rename'}
						</button>
					</div>
				</form>
			{:else if mode === 'delete' && selected}
				<p class="small mb-0">Delete preset “{selected.name}”? Your current {noun} are kept.</p>
				<div class="d-flex justify-content-end gap-2 mt-3">
					<button type="button" class="btn btn-outline-secondary btn-sm" onclick={closeForm}>Cancel</button>
					<button type="button" class="btn btn-danger btn-sm" onclick={confirmDelete}>Delete</button>
				</div>
			{:else if mode === 'import'}
				<form
					onsubmit={(e) => {
						e.preventDefault();
						formError = null;
						runImport(importText);
					}}
				>
					<label class="form-label small mb-1" for="{selectId}-import">
						Paste a preset code, or choose an exported file
					</label>
					<textarea
						id="{selectId}-import"
						class="form-control form-control-sm"
						rows="4"
						spellcheck="false"
						placeholder="Preset code or exported JSON"
						bind:this={importArea}
						bind:value={importText}
						oninput={() => (formError = null)}
					></textarea>
					{#if formError}<div class="form-text text-danger">{formError}</div>{/if}
					<div class="d-flex gap-2 mt-3">
						<button type="button" class="btn btn-outline-secondary btn-sm me-auto" onclick={() => fileInput?.click()}>
							<i class="bi bi-folder2-open"></i> Choose file…
						</button>
						<button type="button" class="btn btn-outline-secondary btn-sm" onclick={closeForm}>Cancel</button>
						<button type="submit" class="btn btn-primary btn-sm" disabled={!importText.trim()}>Import</button>
						<input
							bind:this={fileInput}
							type="file"
							accept=".json,application/json"
							class="d-none"
							aria-label="Preset file"
							onchange={onFile}
						/>
					</div>
				</form>
			{/if}
		</div>
	</dialog>
</div>

<style>
	.preset-manager {
		flex: 1 1 100%;
	}

	.controls {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.75rem 0.75rem;
	}

	.icon-btn {
		text-decoration: none;
	}

	/* Capped so a long label (e.g. "+ 3 unsaved changes") is cut off rather than wrapping the header. */
	.preset-select {
		max-width: 26rem;
		text-overflow: ellipsis;
	}

	/* Same tint as a changed setting row. */
	.changed {
		background-color: rgba(var(--bs-success-rgb), 0.3);
	}

	/* Same tint as a tweaked setting row. */
	.tweaked {
		background-color: rgba(var(--bs-info-rgb), 0.15);
	}

	.menu-wrap {
		position: relative;
	}

	.menu {
		position: absolute;
		top: calc(100% + 0.25rem);
		left: 0;
		z-index: 30;
		min-width: 17rem;
		padding: 0.3rem;
		border: 1px solid var(--bs-border-color);
		border-radius: 0.5rem;
		background: var(--bs-body-bg);
		box-shadow: 0 0.5rem 1.25rem rgb(0 0 0 / 0.35);
	}

	.menu-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.4rem 0.6rem;
		border: 0;
		border-radius: 0.35rem;
		background: none;
		color: var(--bs-body-color);
		font-size: 0.9rem;
		text-align: left;
	}

	.menu-item:hover:not(:disabled),
	.menu-item:focus-visible {
		background: var(--bs-tertiary-bg);
	}

	.menu-item:disabled {
		opacity: 0.45;
	}

	.menu-sep {
		margin: 0.3rem 0;
		border-color: var(--bs-border-color);
		opacity: 1;
	}

	/* Same look as the ROM dialog. */
	.preset-dialog {
		width: min(28rem, calc(100vw - 2rem));
		padding: 0;
		border: 1px solid var(--bs-border-color);
		border-radius: 0.6rem;
		background: var(--bs-body-bg);
		color: var(--bs-body-color);
		font-size: 1rem; /* not the header's smaller text */
		font-weight: normal;
	}

	.preset-dialog::backdrop {
		background: rgb(0 0 0 / 0.55);
	}
</style>
