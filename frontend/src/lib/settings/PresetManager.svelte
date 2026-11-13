<script lang="ts">
	import type { Snippet } from 'svelte';
	import { tooltip } from '$lib/tooltip';
	import type { SettingsStore } from './store.svelte';
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

	// The root preset controls: built-in presets and the player's own, with save, rename, switch, delete,
	// share and import.
	let {
		store,
		kind,
		selectId = 'preset',
		children
	}: {
		store: SettingsStore;
		kind: PresetKind;
		selectId?: string;
		/** Extra controls shown at the end of the row (e.g. a search box). */
		children?: Snippet;
	} = $props();

	type Mode = 'save' | 'rename' | 'switch' | 'delete' | 'import' | null;

	let mode = $state<Mode>(null);
	let menuOpen = $state(false);
	let nameDraft = $state('');
	/** Switch: the preset to switch to. */
	let presetDraft = $state('');
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

	/** The selected preset, if it is one of the player's own. */
	const selected = $derived(store.userPreset(store.selectedId));
	const changeCount = $derived(store.changeCount);
	/** What the dropdown shows while there are unsaved changes: "Mine + 2 unsaved changes". */
	const label = $derived(`${store.presetName} + ${changeCount} unsaved change${changeCount === 1 ? '' : 's'}`);
	/** Value of the hidden entry that shows `label` (not a preset id). */
	const CURRENT = '*current';
	/** What these presets hold, for messages. */
	const noun = $derived(`${kind === 'settings' ? 'generation' : 'customization'} settings`);

	/** "Mine (based on Default)" for the player's own presets when there is more than one built-in. */
	function presetLabel(id: string, name: string): string {
		const preset = store.userPreset(id);
		if (!preset || store.rootPresets.length < 2) return name;
		return `${name} (based on ${store.builtIn(preset.document.preset)?.label ?? store.rootPresets[0].label})`;
	}

	// ---- Menu ---------------------------------------------------------------------
	function onWindowClick(e: MouseEvent) {
		if (menuOpen && menuEl && !menuEl.contains(e.target as Node)) menuOpen = false;
	}

	function onWindowKey(e: KeyboardEvent) {
		if (e.key === 'Escape' && menuOpen) menuOpen = false;
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
		if (next === 'switch') presetDraft = store.selectedId;
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
		if (store.rootPresets.some((p) => sameName(p.label, name))) {
			formError = `“${name}” is a built-in preset name. Please choose another.`;
			return;
		}
		const renaming = mode === 'rename';
		const clash = store.userPresets.find((p) => sameName(p.name, name) && !(renaming && p.id === selected?.id));

		if (renaming) {
			if (!selected) return;
			if (clash) {
				formError = 'You already have a preset with that name.';
				return;
			}
			store.renamePreset(selected.id, name);
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
			store.updatePreset(clash.id);
			closeForm();
			return;
		}

		if (store.userPresets.length >= MAX_USER_PRESETS) {
			formError = `You can keep up to ${MAX_USER_PRESETS} presets. Delete some first.`;
			return;
		}
		store.updatePreset(store.addPreset(name, {}).id);
		closeForm();
	}

	/** Save the current settings: into the selected preset if it is one of the player's own, else as a new one. */
	export function save() {
		if (selected) updateSelected();
		else openMode('save');
	}

	function updateSelected() {
		menuOpen = false;
		if (selected) store.updatePreset(selected.id);
	}

	function confirmSwitch() {
		store.switchPreset(presetDraft);
		closeForm();
	}

	function confirmDelete() {
		if (selected) store.deletePreset(selected.id);
		closeForm();
	}

	// ---- Share: copy code / export file / import ---------------------------------
	async function copyCode() {
		menuOpen = false;
		const name = changeCount ? 'Shared settings' : store.presetName;
		try {
			await navigator.clipboard.writeText(encodePresetCode(kind, name, store.capture()));
		} catch {
			notice = { type: 'danger', text: "Couldn't copy to the clipboard." };
		}
	}

	function exportAll() {
		menuOpen = false;
		const blob = new Blob([buildExportFile(kind, store.userPresets)], { type: 'application/json' });
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
		const result = parseImport(text, kind, (raw) => store.readDocument(raw));
		if (!result.ok) {
			formError = result.error;
			return;
		}
		const room = MAX_USER_PRESETS - store.userPresets.length;
		if (room <= 0) {
			formError = `You already have ${MAX_USER_PRESETS} presets. Delete some first.`;
			return;
		}

		const taken = [...store.rootPresets.map((p) => p.label), ...store.userPresets.map((p) => p.name)];
		const added = [];
		let renamed = 0;
		for (const item of result.presets.slice(0, room)) {
			const name = uniqueName(item.name, taken);
			if (name !== item.name) renamed++;
			taken.push(name);
			added.push(store.addPreset(name, item.document));
		}
		const skipped = result.presets.length - added.length;

		// A single preset is applied straight away so you can see what you imported.
		if (added.length === 1 && !store.locked) store.applyPreset(added[0].id);

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
			class:changed={changeCount > 0}
			disabled={store.locked}
			value={changeCount ? CURRENT : store.selectedId}
			onchange={(e) => store.applyPreset(e.currentTarget.value)}
		>
			{#if changeCount}
				<option value={CURRENT} hidden disabled>{label}</option>
			{/if}
			<optgroup label="Built-in">
				{#each store.rootPresets as p (p.id)}
					<option value={p.id}>{p.label}</option>
				{/each}
			</optgroup>
			{#if store.userPresets.length}
				<optgroup label="My presets">
					{#each store.userPresets as p (p.id)}
						<option value={p.id}>{presetLabel(p.id, p.name)}</option>
					{/each}
				</optgroup>
			{/if}
		</select>

		{#if selected && store.canUpdate}
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
					<button type="button" role="menuitem" class="menu-item" disabled={!selected} onclick={() => openMode('rename')}>
						<i class="bi bi-pencil"></i> Rename…
					</button>
					<button type="button" role="menuitem" class="menu-item" disabled={store.locked} onclick={() => openMode('switch')}>
						<i class="bi bi-arrow-left-right"></i> Switch preset, keeping settings…
					</button>
					<button type="button" role="menuitem" class="menu-item" disabled={!selected} onclick={() => openMode('delete')}>
						<i class="bi bi-trash"></i> Delete…
					</button>
					<hr class="menu-sep" />
					<button type="button" role="menuitem" class="menu-item" onclick={copyCode}>
						<i class="bi bi-clipboard"></i> Copy {noun} as code
					</button>
					<button type="button" role="menuitem" class="menu-item" disabled={!store.userPresets.length} onclick={exportAll}>
						<i class="bi bi-box-arrow-up"></i> Export my presets…
					</button>
					<button type="button" role="menuitem" class="menu-item" onclick={() => openMode('import')}>
						<i class="bi bi-box-arrow-in-down"></i> Import…
					</button>
				</div>
			{/if}
		</div>

		{#if changeCount}
			<button
				type="button"
				class="btn btn-link btn-sm p-0 icon-btn"
				disabled={store.locked}
				use:tooltip={`Put all the ${noun} back to ${store.presetName}.`}
				aria-label="Reset all the {noun}"
				onclick={() => store.resetAll()}
			>
				<i class="bi bi-arrow-counterclockwise"></i>
			</button>
		{/if}

		{@render children?.()}
	</div>

	{#if store.storageFailed}
		<div class="alert alert-danger small py-1 px-2 mt-2 mb-0" role="status">
			Your presets couldn't be saved in this browser (storage may be full or blocked). Export them to keep a backup.
		</div>
	{/if}
	{#if notice}
		<div class="alert alert-{notice.type} small py-1 px-2 mt-2 mb-0 d-flex align-items-center" role="status">
			<span class="me-auto">{notice.text}</span>
			<button type="button" class="btn-close btn-sm" aria-label="Dismiss" onclick={() => (notice = null)}></button>
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
				{mode === 'save' ? 'Save preset' : mode === 'rename' ? 'Rename preset' : mode === 'switch' ? 'Switch preset' : mode === 'delete' ? 'Delete preset' : 'Import presets'}
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
						list={mode === 'save' ? `${selectId}-names` : undefined}
						bind:this={nameInput}
						bind:value={nameDraft}
						oninput={() => {
							overwriteConfirm = false;
							formError = null;
						}}
					/>
					<!-- Suggests your presets' names, for saving over one of them. -->
					<datalist id="{selectId}-names">
						{#each store.userPresets as p (p.id)}
							<option value={p.name}></option>
						{/each}
					</datalist>
					{#if formError}<div class="form-text text-danger">{formError}</div>{/if}
					<div class="d-flex justify-content-end gap-2 mt-3">
						<button type="button" class="btn btn-outline-secondary btn-sm" onclick={closeForm}>Cancel</button>
						<button type="submit" class="btn btn-primary btn-sm">{mode === 'save' ? 'Save' : 'Rename'}</button>
					</div>
				</form>
			{:else if mode === 'switch'}
				<form
					onsubmit={(e) => {
						e.preventDefault();
						confirmSwitch();
					}}
				>
					<label class="form-label small mb-1" for="{selectId}-switch">Switch to</label>
					<select id="{selectId}-switch" class="form-select form-select-sm" bind:value={presetDraft}>
						{#each store.rootPresets as p (p.id)}
							<option value={p.id}>{p.label}</option>
						{/each}
						{#each store.userPresets as p (p.id)}
							<option value={p.id}>{presetLabel(p.id, p.name)}</option>
						{/each}
					</select>
					<div class="form-text">
						Your current {noun} stay as they are; where they differ from the selected preset, they will show as unsaved changes.
					</div>
					<div class="d-flex justify-content-end gap-2 mt-3">
						<button type="button" class="btn btn-outline-secondary btn-sm" onclick={closeForm}>Cancel</button>
						<button type="submit" class="btn btn-primary btn-sm">Switch</button>
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
		field-sizing: content; /* as wide as the option shown, not the widest one (where supported) */
		max-width: 26rem;
		text-overflow: ellipsis;
	}

	/* Same tint as a changed setting row. */
	.changed {
		background-color: rgba(var(--bs-success-rgb), 0.3);
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
