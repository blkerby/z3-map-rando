<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { replaceState } from '$app/navigation';
	import { resolve } from '$app/paths';
	import CustomizationSettings from '$lib/customization/CustomizationSettings.svelte';
	import { customizationModel } from '$lib/customization/model';
	import GenerationSettings from '$lib/generation/GenerationSettings.svelte';
	import { ITEM_LOCATIONS, countPoolItems, generationModel } from '$lib/generation/data';
	import CollapsibleSection from '$lib/settings/CollapsibleSection.svelte';
	import CustomizationPanel from '$lib/settings/CustomizationPanel.svelte';
	import PresetManager from '$lib/settings/PresetManager.svelte';
	import SettingsProvider from '$lib/settings/SettingsProvider.svelte';
	import SummaryPanel from '$lib/settings/SummaryPanel.svelte';
	import { SearchState } from '$lib/settings/search.svelte';
	import { SettingsStore } from '$lib/settings/store.svelte';
	import { pathKey, type Path } from '$lib/settings/tree';
	import { useRomStore } from '$lib/rom/store.svelte';
	import { GENERATOR_VERSION, buildHash, generateGame, parseHash } from '$lib/game';

	const rom = useRomStore();

	// The top-level preset controls, which the side panels' Save buttons go through.
	let settingsPresets = $state<PresetManager>();
	let customPresets = $state<PresetManager>();

	// One search box filters both stores.
	const search = new SearchState();
	const store = new SettingsStore(generationModel, 'gen-', search);
	const custom = new SettingsStore(customizationModel, 'custom-', search);

	/** Reasons the current settings can't be generated. */
	const problems = $derived.by(() => {
		const used = countPoolItems(store);
		if (used <= ITEM_LOCATIONS) return [];
		return [`Item pool: ${used} items is more than the ${ITEM_LOCATIONS} item locations. Lower some counts.`];
	});

	// ---- Phases -------------------------------------------------------------
	// configure: no game yet, settings are editable, Generate section is open.
	// generated: a game exists (seed + settings), settings are locked, Customization is open.
	type Phase = 'configure' | 'generated';
	type Section = 'generate' | 'customization';

	let phase = $state<Phase>('configure');
	let seed = $state<string | null>(null);
	let gameVersion = $state(GENERATOR_VERSION);
	let generating = $state(false);
	let error = $state<string | null>(null);
	let linkError = $state<string | null>(null);
	let ready = $state(false);

	let generateOpen = $state(true);
	let customizationOpen = $state(false);

	const generated = $derived(phase === 'generated');
	const versionNote = $derived(
		generated && gameVersion !== GENERATOR_VERSION
			? `This game was made with generator ${gameVersion}, but this site is running ${GENERATOR_VERSION}. The map may not match.`
			: null
	);
	const shareHash = $derived(generated && seed ? buildHash({ version: gameVersion, seed, settings: store.serialize() }) : null);

	function enterGenerated(newSeed: string, version: string) {
		seed = newSeed;
		gameVersion = version;
		store.locked = true;
		phase = 'generated';
		generateOpen = false;
		customizationOpen = true;
	}

	// ---- Storage ------------------------------------------------------------
	// The URL hash is NOT written while editing. It only exists once a game does,
	// so "has a hash" reliably means "this is a generated game".
	const DRAFT_KEY = 'alttp-maprando:draft';
	const PRESETS_KEY = 'alttp-maprando:presets:settings';
	// Customization is personal: saved in this browser only, never part of the game link, never locked.
	const CUSTOM_KEY = 'alttp-maprando:customization';
	const CUSTOM_PRESETS_KEY = 'alttp-maprando:presets:customization';

	function loadDraft() {
		try {
			const draft = sessionStorage.getItem(DRAFT_KEY);
			if (draft) store.loadDraft(draft);
		} catch {
			/* storage unavailable: start fresh */
		}
	}

	function clearDraft() {
		try {
			sessionStorage.removeItem(DRAFT_KEY);
		} catch {
			/* ignore */
		}
	}

	$effect(() => {
		const draft = store.serializeDraft(); // tracked: re-runs on any change
		if (!ready || phase !== 'configure') return;
		try {
			sessionStorage.setItem(DRAFT_KEY, draft);
		} catch {
			/* ignore */
		}
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
		// Presets before the drafts, which may name one of them.
		store.loadUserPresets(PRESETS_KEY);
		custom.loadUserPresets(CUSTOM_PRESETS_KEY);
		try {
			const draft = localStorage.getItem(CUSTOM_KEY);
			if (draft) custom.loadDraft(draft);
		} catch {
			/* storage unavailable: start from defaults */
		}
		if (location.hash.length > 1) {
			const link = parseHash(location.hash);
			if (link && store.load(link.settings)) {
				enterGenerated(link.seed, link.version);
			} else {
				linkError = "That game link couldn't be read, so it was ignored.";
				loadDraft();
			}
		} else {
			loadDraft();
		}
		ready = true;
	});

	// ---- Actions ------------------------------------------------------------
	async function generate() {
		if (generating || generated) return;
		if (problems.length) return; // the button is disabled too; this covers any other caller
		// ROM first: nothing is requested from the server until the player has a valid ROM loaded.
		if (!(await rom.require())) return;
		if (generating || generated) return; // state may have changed while the dialog was open
		generating = true;
		error = null;
		linkError = null;
		store.locked = true; // freeze the settings while the server works
		try {
			const result = await generateGame($state.snapshot(store.values));
			enterGenerated(result.seed, GENERATOR_VERSION);
			const hash = buildHash({ version: GENERATOR_VERSION, seed: result.seed, settings: store.serialize() });
			replaceState(resolve(`/generate#${hash}`), {});
			clearDraft();
			await tick();
			window.scrollTo({ top: 0, behavior: 'smooth' });
		} catch (e) {
			store.locked = false;
			error = e instanceof Error ? e.message : 'Something went wrong generating the game. Please try again.';
		} finally {
			generating = false;
		}
	}

	// Settings stay as they were, so the player can tweak them and generate again.
	function newGame() {
		seed = null;
		error = null;
		linkError = null;
		store.locked = false;
		phase = 'configure';
		generateOpen = true;
		customizationOpen = false;
		replaceState(resolve('/generate'), {});
		window.scrollTo({ top: 0, behavior: 'smooth' });
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

	// Buttons instead of #anchors, so the URL hash keeps holding the game link.
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
</script>

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

<main class="container my-4">
	{#if linkError}
		<div class="alert alert-warning small py-2 px-3" role="alert">{linkError}</div>
	{/if}
	{#if versionNote}
		<div class="alert alert-warning small py-2 px-3" role="alert">
			<i class="bi bi-exclamation-triangle"></i>
			{versionNote}
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
				bind:open={generateOpen}
			>
				{#snippet actions()}
					<PresetManager bind:this={settingsPresets} {store} kind="settings" selectId="preset" />
				{/snippet}
				<SettingsProvider {store}><GenerationSettings /></SettingsProvider>
				{#if !generateMatches}
					<p class="text-body-secondary">No settings match “{search.query}”.</p>
				{/if}
			</CollapsibleSection>

			<CollapsibleSection id="customization" title="Customization" bind:open={customizationOpen}>
				{#snippet actions()}
					<PresetManager bind:this={customPresets} store={custom} kind="customization" selectId="custom-preset" />
				{/snippet}
				<SettingsProvider store={custom}><CustomizationSettings /></SettingsProvider>
				{#if !customMatches}
					<p class="text-body-secondary">No customization options match “{search.query}”.</p>
				{/if}
			</CollapsibleSection>
		</div>

		<!-- Shared by both sections: stays in view while you scroll through either. -->
		<div class="side">
			<SummaryPanel
				{store}
				{problems}
				{generated}
				{generating}
				{error}
				{seed}
				{shareHash}
				ongenerate={generate}
				onnew={newGame}
				onjump={(path) => jumpToSetting(store, 'generate', path)}
				onsave={() => settingsPresets?.save()}
				onjumpcard={(id) => jumpToCard(store, 'generate', id)}
			/>
			<CustomizationPanel
				store={custom}
				{generated}
				{seed}
				version={gameVersion}
				onjump={(path) => jumpToSetting(custom, 'customization', path)}
				onsave={() => customPresets?.save()}
			/>
		</div>
	</div>
</main>

<style>
	/* Wider than the other pages (see the layout), to give the central column more room. */
	main.container {
		max-width: 1200px;
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
		grid-template-columns: 10.5rem minmax(0, 1fr) 17rem;
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