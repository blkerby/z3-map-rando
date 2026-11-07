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
	let notice = $state<{ type: 'success' | 'info' | 'danger'; text: string } | null>(null);

	let menuEl = $state<HTMLElement>();
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
	/** What the dropdown shows: the preset picked by hand, else "" for the "+ tweaks" entry, else the preset chosen for it. */
	const shown = $derived(store.isSelectionChanged(section) || !tweaks ? current.id : '');
	/** The card's preset was picked by hand: an unsaved change. */
	const picked = $derived(store.isSelectionChanged(section));
	const changed = $derived(store.sectionChanged(section));
	const noun = $derived(card ? `${card.title.toLowerCase()} settings` : kind === 'settings' ? 'settings' : 'customization');

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
			store.adoptSaved(section, clash.id);
			notice = { type: 'success', text: `Updated “${clash.name}”.` };
			closeForm();
			return;
		}

		if (list.userPresets.length >= MAX_USER_PRESETS) {
			formError = `You can keep up to ${MAX_USER_PRESETS} presets. Delete some first.`;
			return;
		}
		const preset = list.add(name, store.capture(section));
		store.adoptSaved(section, preset.id);
		notice = { type: 'success', text: `Saved “${name}”.` };
		closeForm();
	}

	function updateSelected() {
		menuOpen = false;
		if (!selected) return;
		list.update(selected.id, store.capture(section));
		store.adoptSaved(section, selected.id);
		notice = { type: 'success', text: `Updated “${selected.name}” with your current ${noun}.` };
	}

	function confirmDelete() {
		if (!selected) return;
		const name = selected.name;
		store.deletePreset(section, selected.id);
		notice = { type: 'info', text: `Deleted “${name}”. Your current ${noun} were kept.` };
		closeForm();
	}

	// ---- Share: copy code / export file / import ---------------------------------
	async function copyCode() {
		menuOpen = false;
		const name = changed ? 'Shared settings' : current.name;
		try {
			await navigator.clipboard.writeText(encodePresetCode(kind, name, store.capture(section)));
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
			class:picked
			class:tweaked={store.isSelectionTweaked(section)}
			use:tooltip={picked && card ? `Picked by hand. Reset goes back to “${store.inheritedLabel(card)}”.` : undefined}
			disabled={store.locked}
			value={shown}
			onchange={(e) => store.applyPreset(e.currentTarget.value, section)}
		>
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
						<option value={p.id}>{p.name}</option>
					{/each}
				</optgroup>
			{/if}
		</select>
		{#if selected}
			<span class="based-on">based on {list.builtInBehind(selected).name}</span>
		{/if}

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
						disabled={!store.canUpdate(section)}
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
		{#if !card && changed}
			<button
				type="button"
				class="btn btn-sm btn-outline-secondary"
				disabled={store.locked}
				use:tooltip={`Put all the ${noun} back to ${current.name}.`}
				onclick={() => store.resetAll()}
			>
				<i class="bi bi-arrow-counterclockwise"></i> Reset
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

	/* Same tint as a changed setting row. */
	.picked {
		background-color: rgba(var(--bs-success-rgb), 0.3);
	}

	/* Same tint as a tweaked setting row. */
	.tweaked {
		background-color: rgba(var(--bs-info-rgb), 0.15);
	}

	.based-on {
		margin-left: -0.35rem;
		font-size: 0.8rem;
		opacity: 0.65;
		white-space: nowrap;
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
