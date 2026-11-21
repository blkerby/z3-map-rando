<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { loadBrowser } from '$lib/browser';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import CustomizationSettings from '$lib/customization/CustomizationSettings.svelte';
	import { customizationModel } from '$lib/customization/model';
	import GenerationSettings from '$lib/generation/GenerationSettings.svelte';
	import { ITEM_LABELS, ITEM_LOCATIONS, countPoolItems, countStartingItems, generationModel } from '$lib/generation/data';
	import CollapsibleSection from '$lib/settings/CollapsibleSection.svelte';
	import CustomizationPanel from '$lib/settings/CustomizationPanel.svelte';
	import PresetManager from '$lib/settings/PresetManager.svelte';
	import SettingsProvider from '$lib/settings/SettingsProvider.svelte';
	import SummaryPanel from '$lib/settings/SummaryPanel.svelte';
	import { SearchState } from '$lib/settings/search.svelte';
	import { REVISION, StaleError, processDocument, type SettingsIssue } from '$lib/settings/processing';
	import { SettingsStore, type Draft } from '$lib/settings/store.svelte';
	import { pathKey, type Path } from '$lib/settings/tree';
	import { downloadRom, generateGame } from '$lib/game';
	import { useRomStore } from '$lib/rom/store.svelte';

	let {
		seed = null
	}: {
		/** The generated game to show (settings locked), or null to configure a new one. */
		seed?: string | null;
	} = $props();


	// The top-level preset controls, which the side panels' Save buttons go through.
	let settingsPresets = $state<PresetManager>();
	let customPresets = $state<PresetManager>();

	// One search box filters both stores.
	const search = new SearchState();
	const store = new SettingsStore(generationModel, 'gen-', search);
	const custom = new SettingsStore(customizationModel, 'custom-', search);

	/** Reasons the current settings can't be generated. */
	const problems = $derived.by(() => {
		const problems: string[] = [];
		const used = countPoolItems(store);
		if (used > ITEM_LOCATIONS)
			problems.push(`Item pool: ${used} items is more than the ${ITEM_LOCATIONS} item locations. Lower some counts.`);
		if (store.get(['item_pool', 'deduct_starting_items'])) {
			for (const [name, starting] of Object.entries(countStartingItems(store))) {
				const available = store.get(['item_pool', 'items', name]) as number;
				if (starting > available)
					problems.push(`Item pool: the starting state deducts ${starting} ${ITEM_LABELS[name]}, but the pool has only ${available}.`);
			}
		}
		return problems;
	});

	// ---- Modes --------------------------------------------------------------
	// /generate: no game yet, settings are editable, Generate section is open.
	// /seed/<name>: a game exists, its settings are locked, Customization is open.
	type Section = 'generate' | 'customization';

	const generated = $derived(seed !== null);
	const GENERATED_LOCK = 'This game has already been generated, so its settings can no longer be changed.';
	const GENERATING_LOCK = 'Settings are locked while the game is generating.';
	let generating = $state(false);
	let error = $state<string | null>(null);
	/** Passed along by `generate` when it navigates to the seed, so they show only right after generating. */
	const generationNotices = $derived(page.state.notices ?? []);

	// svelte-ignore state_referenced_locally
	let generateOpen = $state(!generated);
	// svelte-ignore state_referenced_locally
	let customizationOpen = $state(generated);

	// ---- Storage ------------------------------------------------------------
	// The draft holds the /generate settings. It is kept after generating, so returning to
	// /generate continues from the settings last used there.
	const DRAFT_KEY = 'alttp-maprando:draft';
	const PRESETS_KEY = 'alttp-maprando:presets:settings';
	// Customization is personal: saved in this browser only, never part of the game link, never locked.
	const CUSTOM_KEY = 'alttp-maprando:customization';
	const CUSTOM_PRESETS_KEY = 'alttp-maprando:presets:customization';

	function saveDraft(revision?: string | null) {
		try {
			sessionStorage.setItem(DRAFT_KEY, store.serializeDraft(revision));
		} catch {
			/* ignore */
		}
	}

	// ---- Loading --------------------------------------------------------------
	// Nothing below the navbar is shown until the stored settings are loaded, so defaults never show in their
	// place. Stored documents saved under another settings revision are processed on the server first, and
	// written back once loading completes; until then, autosave is off and storage is left untouched.

	type LoadState =
		| { status: 'loading' }
		| { status: 'ready' }
		/** The request failed, possibly temporarily. */
		| { status: 'failed'; message: string }
		/** The server runs other settings data than this page. */
		| { status: 'stale' }
		/** No seed of this name. */
		| { status: 'missing' };

	let load = $state<LoadState>({ status: 'loading' });
	/** Loading is taking long enough to show that it is happening. */
	let slow = $state(false);
	/** Stored settings the server couldn't read, which are discarded once the player has seen them. */
	let unreadable = $state<{ name: string; raw: string }[]>([]);
	const ready = $derived(load.status === 'ready' && unreadable.length === 0);

	/** A stored JSON value: null when absent or storage is unavailable, `raw` text when it isn't valid JSON. */
	function readStored(storage: () => Storage, key: string): { value: unknown } | { raw: string } | null {
		let raw: string | null;
		try {
			raw = storage().getItem(key);
		} catch {
			return null;
		}
		if (raw === null) return null;
		try {
			return { value: JSON.parse(raw) };
		} catch {
			return { raw };
		}
	}

	/**
	 * Restore a store's presets and, if given, its draft. The draft and the user preset it selects (which the
	 * editor compares it with) are processed on the server if stored under another revision; other presets
	 * are processed when selected. Returns what couldn't be read, which is left out.
	 */
	async function restore(
		settings: SettingsStore,
		noun: string,
		presetsKey: string,
		draft: { storage: () => Storage; key: string } | null
	): Promise<{ name: string; raw: string }[]> {
		const out = [];
		settings.notices = [];
		const presets = readStored(() => localStorage, presetsKey);
		if (presets && 'raw' in presets) out.push({ name: `Your saved ${noun} presets`, raw: presets.raw });
		settings.loadUserPresets(presets && 'value' in presets ? presets.value : null);
		if (!draft) return out;
		const stored = readStored(draft.storage, draft.key);
		if (!stored) return out;
		if ('raw' in stored) {
			out.push({ name: `Your current ${noun}`, raw: stored.raw });
			return out;
		}
		const saved = stored.value as Draft & { revision: string | null };
		const [presetOutcome, result] = await Promise.all([
			settings.isStale(saved.selected) ? settings.processPreset(saved.selected) : 'processed',
			saved.revision === REVISION ? null : processDocument(settings.model.kind, saved.document)
		]);
		if (presetOutcome === 'unrecoverable') {
			const preset = settings.userPreset(saved.selected)!;
			out.push({ name: `Preset “${preset.name}”`, raw: JSON.stringify(preset.document) });
			settings.deletePreset(preset.id);
		}
		if (result?.status === 'unrecoverable') {
			out.push({ name: `Your current ${noun}`, raw: JSON.stringify(saved.document) });
			return out;
		}
		if (result?.notices.length) settings.notices.push({ source: `Your current ${noun}`, issues: result.notices });
		settings.loadDraft(result ? { ...saved, document: result.document } : saved);
		return out;
	}

	/** Load the seed's settings into the (locked) store. Returns false if there is no such seed. */
	async function loadSeed(name: string): Promise<boolean> {
		let response: Response;
		try {
			response = await fetch(`/api/seed/${encodeURIComponent(name)}`);
		} catch {
			throw new Error("Couldn't reach the server.");
		}
		if (response.status === 404) return false;
		if (!response.ok) throw new Error(`Couldn't load the seed (server error ${response.status}).`);
		const data = await response.json();
		store.loadSeed(store.readDocument(data.settings_document));
		store.lockReason = GENERATED_LOCK;
		return true;
	}

	async function loadPage() {
		load = { status: 'loading' };
		slow = false;
		const timer = setTimeout(() => (slow = true), 300);
		try {
			const [found, generationUnreadable, customUnreadable] = await Promise.all([
				seed === null || loadSeed(seed),
				restore(store, 'generation settings', PRESETS_KEY, seed === null ? { storage: () => sessionStorage, key: DRAFT_KEY } : null),
				restore(custom, 'customization', CUSTOM_PRESETS_KEY, { storage: () => localStorage, key: CUSTOM_KEY })
			]);
			unreadable = [...generationUnreadable, ...customUnreadable];
			load = found ? { status: 'ready' } : { status: 'missing' };
		} catch (e) {
			if (e instanceof StaleError) showStale();
			else load = { status: 'failed', message: e instanceof Error ? e.message : "Couldn't load your settings." };
		} finally {
			clearTimeout(timer);
		}
	}

	/**
	 * The server runs other settings data than this page: replace the content with the prompt to reload. This
	 * also stops autosave, so the stale page writes nothing further.
	 */
	function showStale() {
		load = { status: 'stale' };
	}

	async function copyUnreadable() {
		const text = unreadable.map((item) => `${item.name}:\n${item.raw}`).join('\n\n');
		try {
			await navigator.clipboard.writeText(text);
		} catch {
			/* nothing to fall back to */
		}
	}

	/** A notice line: the setting's label, if it has a control, and what happened. */
	function noticeText(settings: SettingsStore, issue: SettingsIssue): string {
		return issue.path.length ? `${settings.label(issue.path)}: ${issue.message}` : issue.message;
	}

	$effect(() => {
		store.serializeDraft(); // tracked: re-runs on any change
		if (ready && !generated) saveDraft();
	});

	$effect(() => {
		const draft = custom.serializeDraft(); // tracked: re-runs on any change
		if (!ready) return;
		try {
			localStorage.setItem(CUSTOM_KEY, draft);
		} catch {
			/* ignore */
		}
	});

	$effect(() => {
		JSON.stringify(store.userPresets); // tracked: re-runs when any preset changes
		if (ready) store.saveUserPresets(PRESETS_KEY);
	});

	$effect(() => {
		JSON.stringify(custom.userPresets); // tracked: re-runs when any preset changes
		if (ready) custom.saveUserPresets(CUSTOM_PRESETS_KEY);
	});

	onMount(() => {
		void loadBrowser().catch((e) => {
			error = e instanceof Error ? e.message : 'Could not load browser settings support.';
		});
		void loadPage();
	});

	// ---- Actions ------------------------------------------------------------
	async function generate() {
		if (generating || generated) return;
		if (problems.length) return; // the button is disabled too; this covers any other caller
		generating = true;
		error = null;
		store.lockReason = GENERATING_LOCK; // freeze the settings while the server works
		try {
			const result = await generateGame(store.captureDocument());
			await goto(resolve('/seed/[name]', { name: result.seed }), { state: { notices: result.notices } });
		} catch (e) {
			store.lockReason = null;
			error = e instanceof Error ? e.message : 'Something went wrong generating the game. Please try again.';
		} finally {
			generating = false;
		}
	}

	const rom = useRomStore();
	let downloading = $state(false);
	let romError = $state<string | null>(null);

	async function download() {
		if (!seed || downloading) return;
		if (!(await rom.require())) return;
		downloading = true;
		romError = null;
		try {
			await downloadRom({ seed, rom: rom.bytes!, customization: $state.snapshot(custom.values) });
		} catch (e) {
			romError = e instanceof Error ? e.message : 'Could not download the ROM.';
		} finally {
			downloading = false;
		}
	}

	/**
	 * Make this game's settings the /generate draft, so the player can tweak them and generate again. They
	 * were saved under the revision that generated the game, so the draft is processed when loaded.
	 */
	function useSettingsForNewGame() {
		store.releaseSeed();
		saveDraft(null);
		goto(resolve('/generate'));
	}

	// ---- Search (covers both sections) --------------------------------------
	const searching = $derived(search.query.trim().length > 0);
	const generateMatches = $derived(store.countMatches());
	const customMatches = $derived(custom.countMatches());

	// Open any section that has results. Only re-runs when the query or the results change,
	// so collapsing a section by hand while searching sticks until the next keystroke.
	$effect(() => {
		if (!searching) return;
		if (generateMatches > 0) generateOpen = true;
		if (customMatches > 0) customizationOpen = true;
	});

	// ---- Navigation ---------------------------------------------------------
	let activeId = $state('');

	function openSection(section: Section) {
		if (section === 'generate') generateOpen = true;
		else customizationOpen = true;
	}

	// Buttons instead of #anchors, so the URL stays a clean game link.
	async function goTo(section: Section, elementId: string) {
		openSection(section);
		activeId = elementId;
		await tick(); // the section may have been collapsed
		document.getElementById(elementId)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
	}

	function scrollToTop() {
		activeId = '';
		window.scrollTo({ top: 0, behavior: 'smooth' });
	}

	/** Clear the search, open a card and scroll to it. */
	function jumpToCard(settings: SettingsStore, section: Section, cardId: string) {
		search.query = '';
		settings.opened[cardId] = true;
		goTo(section, `${settings.idPrefix}section-${cardId}`);
	}

	/** From a summary chip: clear the search so the setting is visible, open its card, scroll to it. */
	async function jumpToSetting(settings: SettingsStore, section: Section, path: Path) {
		search.query = '';
		openSection(section);
		const cardId = settings.controls[pathKey(path)]?.cardId;
		if (cardId) settings.opened[cardId] = true;
		await tick();
		document.getElementById(settings.elementId(path))?.scrollIntoView({ behavior: 'smooth', block: 'center' });
	}

	// ---- Lock toast ---------------------------------------------------------
	// Pressing a control that the lock disables (marked `data-lockable`, directly or on a wrapper) shows
	// the lock reason in a toast for a few seconds. The nearest marker decides, so `data-lockable="false"`
	// exempts a part (e.g. a help button) of a larger lockable area. Only the Generate section's controls lock; the
	// Customization ones carry the same marker but stay editable. Pointerdown rather than click, since browsers don't
	// fire click on disabled controls.
	let lockToast = $state(false);
	let lockToastTimer: ReturnType<typeof setTimeout> | undefined;

	function onWindowPointerDown(e: PointerEvent) {
		const marked = (e.target as Element).closest<HTMLElement>('#generate [data-lockable]');
		if (!store.locked || !marked || marked.dataset.lockable === 'false') return;
		lockToast = true;
		clearTimeout(lockToastTimer);
		lockToastTimer = setTimeout(() => (lockToast = false), 3000);
	}
</script>

<svelte:window onpointerdown={onWindowPointerDown} />

<!-- Links to a store's cards. -->
{#snippet navLinks(settings: SettingsStore, page: Section)}
	{#each settings.cards as card (card.id)}
		{#if !searching || settings.countMatches(card.id)}
			<button
				type="button"
				class="nav-link-btn sub"
				class:active={activeId === `${settings.idPrefix}section-${card.id}`}
				onclick={() => jumpToCard(settings, page, card.id)}
			>
				{#if card.icon}<i class="bi bi-{card.icon}"></i>{/if}
				{card.title}
			</button>
		{/if}
	{/each}
{/snippet}

{#if !ready}
	<main class="container my-4">
		{#if load.status === 'loading'}
			{#if slow}
				<div class="d-flex align-items-center gap-2 text-body-secondary" role="status">
					<span class="spinner-border spinner-border-sm" aria-hidden="true"></span>
					Loading your settings…
				</div>
			{/if}
		{:else if load.status === 'failed'}
			<div class="alert alert-danger d-flex align-items-center gap-3" role="alert">
				<span class="me-auto">{load.message}</span>
				<button type="button" class="btn btn-outline-danger btn-sm" onclick={loadPage}>Retry</button>
			</div>
		{:else if load.status === 'stale'}
			<div class="alert alert-warning d-flex align-items-center gap-3" role="alert">
				<span class="me-auto">The randomizer has been updated. Reload the page to continue.</span>
				<button type="button" class="btn btn-outline-secondary btn-sm" onclick={() => location.reload()}>Reload</button>
			</div>
		{:else if load.status === 'missing'}
			<div class="alert alert-warning" role="alert">There is no game called “{seed}”.</div>
		{:else}
			<div class="card p-4 unreadable" role="alert">
				<h2 class="h5 fw-bold mb-2">Some saved settings can’t be read</h2>
				<p class="small mb-2">
					This is a bug. These will be discarded; copying them first lets you include them in a report.
					Please let us know in the <a href="https://discord.gg/Mxb5zYZeVj" target="_blank" rel="noopener">Discord</a>.
				</p>
				<ul class="small">
					{#each unreadable as item (item.name)}
						<li>{item.name}</li>
					{/each}
				</ul>
				<div class="d-flex gap-2">
					<button type="button" class="btn btn-outline-secondary btn-sm me-auto" onclick={copyUnreadable}>
						<i class="bi bi-clipboard"></i> Copy settings
					</button>
					<button type="button" class="btn btn-primary btn-sm" onclick={() => (unreadable = [])}>Discard and continue</button>
				</div>
			</div>
		{/if}
	</main>
{:else}
<main class="container my-4">
	{#each [store, custom] as settings (settings.idPrefix)}
		{#if settings.notices.length}
			<div class="alert alert-info small py-2 px-3 d-flex align-items-start" role="status">
				<div class="me-auto">
					<div class="fw-semibold">Some saved settings were updated for this version of the randomizer.</div>
					{#each settings.notices as group, i (i)}
						<div class="mt-1">{group.source}:</div>
						<ul class="mb-0">
							{#each group.issues as issue, j (j)}
								<li>{noticeText(settings, issue)}</li>
							{/each}
						</ul>
					{/each}
				</div>
				<button type="button" class="btn-close btn-sm" aria-label="Dismiss" onclick={() => (settings.notices = [])}></button>
			</div>
		{/if}
	{/each}

	{#if generationNotices.length}
		<div class="alert alert-warning small py-2 px-3" role="status">
			{#each generationNotices as notice}
				<div>{notice}</div>
			{/each}
		</div>
	{/if}

	<div class="searchbar mb-3">
		<input
			type="search"
			class="form-control search"
			placeholder="Search all settings…"
			aria-label="Search settings"
			bind:value={search.query}
		/>
		{#if searching}
			<span class="small text-body-secondary" role="status">
				{generateMatches} in Generate · {customMatches} in Customization
			</span>
		{/if}
	</div>

	<div class="layout">
		<nav class="page-nav" aria-label="Page sections">
			<button type="button" class="nav-link-btn top-link" onclick={scrollToTop}>
				<i class="bi bi-arrow-up"></i>
				Top
			</button>

			<button
				type="button"
				class="nav-link-btn section-link"
				class:active={activeId === 'generate'}
				onclick={() => goTo('generate', 'generate')}
			>
				Generate
			</button>
			{@render navLinks(store, 'generate')}

			<button
				type="button"
				class="nav-link-btn section-link"
				class:active={activeId === 'customization'}
				onclick={() => goTo('customization', 'customization')}
			>
				Customization
			</button>
			{@render navLinks(custom, 'customization')}
		</nav>

		<div class="content">
			<CollapsibleSection
				id="generate"
				title="Generate"
				badge={generated ? 'Locked' : undefined}
				badgeIcon="lock-fill"
				badgeTooltip={GENERATED_LOCK}
				bind:open={generateOpen}
			>
				{#snippet actions()}
					<PresetManager bind:this={settingsPresets} {store} kind="settings" selectId="preset" onstale={showStale} />
				{/snippet}
				<SettingsProvider {store}><GenerationSettings /></SettingsProvider>
				{#if !generateMatches}
					<p class="text-body-secondary">No settings match “{search.query}”.</p>
				{/if}
				{#if error}
					<div class="alert alert-danger small py-2 px-3" role="alert">{error}</div>
				{/if}
				{#each problems as problem (problem)}
					<div class="alert alert-warning small py-2 px-3" role="alert">{problem}</div>
				{/each}
				{#if !generated}
					<button type="button" class="btn btn-primary px-4 py-2" disabled={generating || problems.length > 0} onclick={generate}>
						{#if generating}
							<span class="spinner-border spinner-border-sm me-1" aria-hidden="true"></span>
							Generating…
						{:else}
							<i class="bi bi-dice-5"></i> Generate game
						{/if}
					</button>
				{/if}
			</CollapsibleSection>

			<CollapsibleSection id="customization" title="Customization" bind:open={customizationOpen}>
				{#snippet actions()}
					<PresetManager bind:this={customPresets} store={custom} kind="customization" selectId="custom-preset" onstale={showStale} />
				{/snippet}
				<SettingsProvider store={custom}><CustomizationSettings /></SettingsProvider>
				{#if !customMatches}
					<p class="text-body-secondary">No customization options match “{search.query}”.</p>
				{/if}
				{#if romError}
					<div class="alert alert-danger small py-2 px-3" role="alert">{romError}</div>
				{/if}
				<span class="d-inline-block mt-3" title={generated ? undefined : 'Generate a game first, before a ROM can be saved.'}>
					<button type="button" class="btn btn-primary px-4 py-2" disabled={!generated || downloading} onclick={download}>
						{#if downloading}
							<span class="spinner-border spinner-border-sm me-1" aria-hidden="true"></span>
							Preparing…
						{:else}
							<i class="bi bi-download"></i> Save ROM
						{/if}
					</button>
				</span>
			</CollapsibleSection>
		</div>

		<!-- Shared by both sections: stays in view while you scroll through either. -->
		<div class="side">
			<SummaryPanel
				{store}
				{generated}
				{seed}
				onreuse={useSettingsForNewGame}
				onjump={(path) => jumpToSetting(store, 'generate', path)}
				onsave={() => settingsPresets?.save()}
				onjumpcard={(id) => jumpToCard(store, 'generate', id)}
			/>
			<CustomizationPanel
				store={custom}
				onjump={(path) => jumpToSetting(custom, 'customization', path)}
				onsave={() => customPresets?.save()}
			/>
		</div>
	</div>
</main>
{/if}

{#if lockToast}
	<div class="toast show lock-toast" role="status">
		<div class="toast-body d-flex align-items-center gap-2">
			<i class="bi bi-lock-fill" aria-hidden="true"></i>
			{store.lockReason}
		</div>
	</div>
{/if}

<style>
	.unreadable {
		max-width: 36rem;
	}

	.lock-toast {
		position: fixed;
		bottom: 1.5rem;
		left: 50%;
		transform: translateX(-50%);
		z-index: 1090;
		max-width: calc(100vw - 2rem);
		pointer-events: none;
	}

	.searchbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 1rem;
	}

	.search {
		max-width: 26rem;
	}

	.layout {
		display: grid;
		grid-template-columns: 10.5rem minmax(0, 1fr) 20rem;
		gap: 1.25rem;
		align-items: start;
	}

	.page-nav {
		position: sticky;
		top: 1rem;
		max-height: calc(100vh - 2rem);
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}

	.nav-link-btn {
		display: block;
		width: 100%;
		padding: 0.35rem 0.6rem;
		border: 0;
		border-radius: 0.4rem;
		background: none;
		color: var(--bs-body-color);
		font: inherit;
		text-align: left;
	}

	.nav-link-btn:hover,
	.nav-link-btn.active {
		background: var(--bs-tertiary-bg);
	}

	.top-link {
		margin-bottom: 0.4rem;
		border-bottom: 1px solid var(--bs-border-color);
		border-radius: 0.4rem 0.4rem 0 0;
		opacity: 0.8;
	}

	.section-link {
		margin-top: 0.6rem;
		font-weight: 600;
		color: var(--bs-emphasis-color);
	}

	.sub {
		font-size: 0.92rem;
	}

	/* Both panels stick together: Generation settings on top, Customization settings beneath. */
	.side {
		position: sticky;
		top: 1rem;
		max-height: calc(100vh - 2rem);
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	:global(#generate),
	:global(#customization) {
		scroll-margin-top: 1rem;
	}

	@media (max-width: 992px) {
		/* Capped so setting controls stay near their labels. */
		main.container {
			max-width: 44rem;
		}

		.layout {
			grid-template-columns: minmax(0, 1fr);
		}

		.page-nav {
			display: none;
		}

		.side {
			order: -1;
			position: static;
			max-height: none;
			overflow: visible;
		}
	}
</style>
