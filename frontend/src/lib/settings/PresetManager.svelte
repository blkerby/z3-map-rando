<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import { tooltip } from '$lib/tooltip';
	import type { SettingsStore } from './store.svelte';
	import type { Section } from './types';
	import {
		FORMAT_VERSION,
		MAX_IMPORT_BYTES,
		MAX_NAME,
		MAX_USER_PRESETS,
		buildExportFile,
		cleanName,
		encodePresetCode,
		parseImport,
		readStoredPresets,
		sameName,
		uniqueName,
		type PresetKind
	} from './presetIO';

	let {
		store,
		section,
		kind,
		storageKey,
		selectId = 'preset',
		children
	}: {
		store: SettingsStore;
		/** Manage this card's own presets instead of the presets for all the settings. */
		section?: Section;
		kind: PresetKind;
		/** localStorage key for this list of presets. */
		storageKey: string;
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
	let notice = $state<{ type: 'success' | 'info' | 'danger'; text: string } | null>(null);
	let ready = $state(false);

	let menuEl = $state<HTMLElement>();
	let nameInput = $state<HTMLInputElement>();
	let importArea = $state<HTMLTextAreaElement>();
	let fileInput = $state<HTMLInputElement>();

	const list = $derived(store.listFor(section));
	/** The selected preset, if it is one of the player's own. */
	const selected = $derived(list.userPresets.find((p) => p.id === list.selectedId));
	const hasChanges = $derived(store.hasChanges(section));
	const noun = $derived(section ? `${section.title.toLowerCase()} settings` : kind === 'settings' ? 'settings' : 'customization');

	// ---- Persistence: the player's presets live in this browser ------------------
	onMount(() => {
		try {
			const raw = localStorage.getItem(storageKey);
			if (raw) list.userPresets = readStoredPresets(raw, (v) => store.sanitize(v, section));
		} catch {
			/* storage unavailable: presets just won't persist */
		}
		ready = true;
	});

	$effect(() => {
		const data = JSON.stringify({ version: FORMAT_VERSION, presets: list.userPresets }); // tracked
		if (!ready) return;
		try {
			localStorage.setItem(storageKey, data);
		} catch {
			notice = {
				type: 'danger',
				text: "Your presets couldn't be saved in this browser (storage may be full or blocked). Export them to keep a backup."
			};
		}
	});

	// ---- Menu ---------------------------------------------------------------------
	function onWindowClick(e: MouseEvent) {
		if (menuOpen && menuEl && !menuEl.contains(e.target as Node)) menuOpen = false;
	}

	function onWindowKey(e: KeyboardEvent) {
		if (e.key !== 'Escape') return;
		if (menuOpen) menuOpen = false;
		else if (mode) closeForm();
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

	$effect(() => {
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
			notice = { type: 'success', text: `Renamed to “${name}”.` };
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
			list.selectedId = clash.id;
			notice = { type: 'success', text: `Updated “${clash.name}”.` };
			closeForm();
			return;
		}

		if (list.userPresets.length >= MAX_USER_PRESETS) {
			formError = `You can keep up to ${MAX_USER_PRESETS} presets. Delete some first.`;
			return;
		}
		const preset = list.add(name, store.capture(section));
		list.selectedId = preset.id;
		notice = { type: 'success', text: `Saved “${name}”.` };
		closeForm();
	}

	function updateSelected() {
		menuOpen = false;
		if (!selected) return;
		list.update(selected.id, store.capture(section));
		notice = { type: 'success', text: `Updated “${selected.name}” with your current ${noun}.` };
	}

	function confirmDelete() {
		if (!selected) return;
		const name = selected.name;
		list.delete(selected.id);
		notice = { type: 'info', text: `Deleted “${name}”. Your current ${noun} were kept.` };
		closeForm();
	}

	// ---- Share: copy code / export file / import ---------------------------------
	async function copyCode() {
		menuOpen = false;
		const name = hasChanges ? 'Shared settings' : list.selected.name;
		try {
			await navigator.clipboard.writeText(encodePresetCode(kind, name, store.diff(section), store.capture(section).groupPresets));
			notice = { type: 'success', text: 'Preset code copied. Anyone can paste it into Import.' };
		} catch {
			notice = { type: 'danger', text: "Couldn't copy to the clipboard." };
		}
	}

	function exportAll() {
		menuOpen = false;
		if (!list.userPresets.length) {
			notice = { type: 'info', text: 'You have no saved presets to export yet.' };
			return;
		}
		const blob = new Blob([buildExportFile(kind, list.userPresets)], { type: 'application/json' });
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = `alttp-maprando-${kind}-presets.json`;
		document.body.append(a);
		a.click();
		a.remove();
		setTimeout(() => URL.revokeObjectURL(url), 1000);
		const n = list.userPresets.length;
		notice = { type: 'success', text: `Exported ${n} preset${n === 1 ? '' : 's'}.` };
	}

	function runImport(text: string) {
		const result = parseImport(text, kind, (raw) => store.sanitize(raw, section));
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
			added.push(list.add(name, { values: item.values, groupPresets: section ? undefined : item.groupPresets }));
		}
		const skipped = result.presets.length - added.length;

		// A single preset is applied straight away so you can see what you imported.
		if (added.length === 1 && !store.locked) store.applyPreset(added[0].id, section);

		let text2 = `Imported ${added.length} preset${added.length === 1 ? '' : 's'}`;
		if (renamed) text2 += ` (${renamed} renamed to avoid duplicates)`;
		if (skipped) text2 += `; ${skipped} skipped because the list is full`;
		notice = { type: 'success', text: text2 + '.' };
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
		<label class="fw-medium" for={selectId}>Preset</label>
		<select
			id={selectId}
			class="form-select form-select-sm w-auto"
			disabled={store.locked}
			value={list.selectedId}
			onchange={(e) => store.applyPreset(e.currentTarget.value, section)}
		>
			<optgroup label="Built-in">
				{#each list.builtIn as p (p.id)}
					<option value={p.id}>{p.name}</option>
				{/each}
			</optgroup>
			{#if list.userPresets.length}
				<optgroup label="My presets">
					{#each list.userPresets as p (p.id)}
						<option value={p.id}>{p.name}</option>
					{/each}
				</optgroup>
			{/if}
		</select>

		<div class="menu-wrap" bind:this={menuEl}>
			<button
				type="button"
				class="btn btn-outline-secondary btn-sm"
				aria-haspopup="menu"
				aria-expanded={menuOpen}
				onclick={() => (menuOpen = !menuOpen)}
			>
				<i class="bi bi-three-dots"></i> Manage
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
						disabled={!selected || !hasChanges}
						onclick={updateSelected}
					>
						<i class="bi bi-arrow-repeat"></i>
						{selected ? `Update “${selected.name}”` : 'Update preset'}
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
		{#if !section && hasChanges}
			<button
				type="button"
				class="btn btn-sm btn-outline-secondary"
				disabled={store.locked}
				use:tooltip={`Put all the ${noun} back to ${list.selected.name}.`}
				onclick={() => store.applyPreset(list.selectedId)}
			>
				<i class="bi bi-arrow-counterclockwise"></i> Reset
			</button>
		{/if}

		{@render children?.()}
	</div>

	{#if notice}
		<div class="alert alert-{notice.type} small py-1 px-2 mt-2 mb-0 d-flex align-items-center" role="status">
			<span class="me-auto">{notice.text}</span>
			<button type="button" class="btn-close btn-sm" aria-label="Dismiss" onclick={() => (notice = null)}
			></button>
		</div>
	{/if}

	{#if mode === 'save' || mode === 'rename'}
		<form
			class="inline-form"
			onsubmit={(e) => {
				e.preventDefault();
				submitName();
			}}
		>
			<label class="form-label small mb-1" for="{selectId}-name">
				{mode === 'save' ? `Save your current ${noun} as…` : 'Rename preset'}
			</label>
			<div class="d-flex flex-wrap gap-2">
				<input
					id="{selectId}-name"
					class="form-control form-control-sm name-input"
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
				<button type="submit" class="btn btn-primary btn-sm">
					{mode === 'save' ? 'Save' : 'Rename'}
				</button>
				<button type="button" class="btn btn-outline-secondary btn-sm" onclick={closeForm}>Cancel</button>
			</div>
			{#if formError}<div class="form-text text-danger">{formError}</div>{/if}
		</form>
	{:else if mode === 'delete' && selected}
		<div class="inline-form d-flex flex-wrap align-items-center gap-2">
			<span class="small">Delete preset “{selected.name}”?</span>
			<button type="button" class="btn btn-danger btn-sm" onclick={confirmDelete}>Delete</button>
			<button type="button" class="btn btn-outline-secondary btn-sm" onclick={closeForm}>Cancel</button>
		</div>
	{:else if mode === 'import'}
		<form
			class="inline-form"
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
				class="form-control form-control-sm mb-2"
				rows="3"
				spellcheck="false"
				placeholder="Preset code or exported JSON"
				bind:this={importArea}
				bind:value={importText}
				oninput={() => (formError = null)}
			></textarea>
			<div class="d-flex flex-wrap gap-2">
				<button type="submit" class="btn btn-primary btn-sm" disabled={!importText.trim()}>Import</button>
				<button type="button" class="btn btn-outline-secondary btn-sm" onclick={() => fileInput?.click()}>
					<i class="bi bi-folder2-open"></i> Choose file…
				</button>
				<button type="button" class="btn btn-outline-secondary btn-sm" onclick={closeForm}>Cancel</button>
				<input
					bind:this={fileInput}
					type="file"
					accept=".json,application/json"
					class="d-none"
					aria-label="Preset file"
					onchange={onFile}
				/>
			</div>
			{#if formError}<div class="form-text text-danger">{formError}</div>{/if}
		</form>
	{/if}
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

	.inline-form {
		margin-top: 0.6rem;
		padding: 0.6rem 0.75rem;
		border: 1px solid var(--bs-border-color);
		border-radius: 0.5rem;
		background: var(--bs-tertiary-bg);
	}

	.name-input {
		max-width: 18rem;
	}
</style>
