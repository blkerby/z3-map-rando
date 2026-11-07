<script lang="ts">
	import { tooltip } from '$lib/tooltip';
	import { onMount, tick } from 'svelte';
	import { replaceState } from '$app/navigation';
	import { resolve } from '$app/paths';
	import CollapsibleSection from '$lib/settings/CollapsibleSection.svelte';
	import CustomizationEditor from '$lib/settings/CustomizationEditor.svelte';
	import CustomizationPanel from '$lib/settings/CustomizationPanel.svelte';
	import PresetManager from '$lib/settings/PresetManager.svelte';
	import SettingRow from '$lib/settings/SettingRow.svelte';
	import SummaryPanel from '$lib/settings/SummaryPanel.svelte';
	import { customizationGroups, customizationPresets } from '$lib/settings/customizationSchema';
	import { buildSchema } from '$lib/master/adapter';
	import type { PageData } from './$types';
	import { SearchState } from '$lib/settings/search.svelte';
	import { SettingsStore } from '$lib/settings/store.svelte';
	import type { Setting } from '$lib/settings/types';
	import { useRomStore } from '$lib/rom/store.svelte';
	import { GENERATOR_VERSION, buildHash, generateGame, parseHash } from '$lib/game';

	const rom = useRomStore();

	let { data }: { data: PageData } = $props();

	// The settings description is loaded once per visit (see +page.ts), so it is read once here.
	// svelte-ignore state_referenced_locally
	const schema = buildSchema(data.master);

	// One search box filters both stores.
	const search = new SearchState();
	const store = new SettingsStore(schema.groups, schema.presets, search);
	const custom = new SettingsStore(customizationGroups, customizationPresets, search);

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
	const shareHash = $derived(
		generated && seed
			? buildHash({ version: gameVersion, seed, settings: store.serialize() })
			: null
	);

	function enterGenerated(newSeed: string, version: string) {
		seed = newSeed;
		gameVersion = version;
		store.locked = true;
		phase = 'generated';
		generateOpen = false;
		customizationOpen = true;
	}

	// ---- Draft: survive a refresh while editing -----------------------------
	// The URL hash is NOT written while editing. It only exists once a game does,
	// so "has a hash" reliably means "this is a generated game".
	const DRAFT_KEY = 'alttp-maprando:draft';

	function loadDraft() {
		try {
			const code = sessionStorage.getItem(DRAFT_KEY);
			if (code) store.load(code);
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
		const code = store.serialize({ keepPreset: true }); // tracked: re-runs on any change
		if (!ready || phase !== 'configure') return;
		try {
			sessionStorage.setItem(DRAFT_KEY, code);
		} catch {
			/* ignore */
		}
	});

	onMount(() => {
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
		if (store.problems.length) return; // the button is disabled too; this covers any other caller
		// ROM first: nothing is requested from the server until the player has a valid ROM loaded.
		if (!(await rom.require())) return;
		if (generating || generated) return; // state may have changed while the dialog was open
		generating = true;
		error = null;
		linkError = null;
		store.locked = true; // freeze the settings while the server works
		try {
			// Hidden settings are not shown or put in links, but the server still needs their fixed values.
			const result = await generateGame({ ...schema.hidden, ...$state.snapshot(store.values) });
			enterGenerated(result.seed, GENERATOR_VERSION);
			const hash = buildHash({
				version: GENERATOR_VERSION,
				seed: result.seed,
				settings: store.serialize()
			});
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
	const generateMatches = $derived(store.visibleGroups.reduce((n, x) => n + x.rows.length, 0));
	const customMatches = $derived(custom.visibleGroups.reduce((n, x) => n + x.rows.length, 0));

	// Open any section that has results. Only re-runs when the query or the results change,
	// so collapsing a section by hand while searching sticks until the next keystroke.
	$effect(() => {
		if (!searching) return;
		if (generateMatches > 0) generateOpen = true;
		if (customMatches > 0) customizationOpen = true;
	});

	// ---- Blocks: runs of rows under one heading, laid out as a list or a grid ----
	type Block = { key: string; path: string[]; layout: 'list' | 'grid' | 'compact'; rows: Setting[] };

	function blocksOf(groupId: string, rows: Setting[]): Block[] {
		const out: Block[] = [];
		for (const s of rows) {
			const last = out[out.length - 1];
			if (last && last.key === `${groupId}|${s.heading ?? ''}`) {
				last.rows.push(s);
			} else {
				out.push({ key: `${groupId}|${s.heading ?? ''}`, path: s.headingPath ?? [], layout: s.layout ?? 'list', rows: [s] });
			}
		}
		return out;
	}

	// Cards (like Items) the player has opened or folded by hand, by group id. Cards not in here use
	// their default: open, unless the master file says `collapsed`. Searching opens every card with a match.
	let cardOpen = $state<Record<string, boolean>>({});

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
		if (elementId.startsWith('group-')) cardOpen[elementId.slice('group-'.length)] = true; // jumping to a folded card opens it
		await tick(); // the section may have been collapsed
		document.getElementById(elementId)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
	}

	function scrollToTop() {
		activeId = '';
		window.scrollTo({ top: 0, behavior: 'smooth' });
	}

	/** From a summary chip: clear the search so the setting is visible, open its section, scroll to it. */
	async function jumpToSetting(section: Section, id: string) {
		search.query = '';
		openSection(section);
		// If the setting sits in a folded card, unfold it first.
		const owner = store.groups.find((g) => g.settings.some((x) => x.id === id));
		if (owner) cardOpen[owner.id] = true;
		await tick();
		// Nested settings live inside a collapsed expander; jumping to the top-level parent is enough.
		(document.getElementById(id) ?? document.getElementById(id.split('.')[0]))?.scrollIntoView({ behavior: 'smooth', block: 'center' });
	}
</script>

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
			{#each store.visibleGroups as { group } (group.id)}
				<button
					type="button"
					class="nav-link-btn sub"
					class:active={activeId === `group-${group.id}`}
					onclick={() => goTo('generate', `group-${group.id}`)}
				>
					{#if group.icon}<i class="bi bi-{group.icon}"></i>{/if}
					{group.title}
				</button>
			{/each}

			<button
				type="button"
				class="nav-link-btn section-link"
				class:active={activeId === 'customization'}
				onclick={() => goTo('customization', 'customization')}
			>
				Customization
			</button>
			{#each custom.visibleGroups as { group } (group.id)}
				<button
					type="button"
					class="nav-link-btn sub"
					class:active={activeId === `custom-group-${group.id}`}
					onclick={() => goTo('customization', `custom-group-${group.id}`)}
				>
					{#if group.icon}<i class="bi bi-{group.icon}"></i>{/if}
					{group.title}
				</button>
			{/each}
		</nav>

		<div class="content">
			<CollapsibleSection
				id="generate"
				title="Generate"
				badge={generated ? 'Locked' : undefined}
				badgeIcon="lock-fill"
				bind:open={generateOpen}
			>
				<div class="mb-3">
					<PresetManager {store} kind="settings" storageKey="alttp-maprando:presets:settings" selectId="preset" />
				</div>

				{#each store.visibleGroups as { group, rows } (group.id)}
					{@const open = !group.collapsible || searching || (cardOpen[group.id] ?? !group.collapsed)}
					{@const pool = store.poolOf(group)}
					<section id="group-{group.id}" class="card mb-3">
						<header class="card-header d-flex align-items-center" class:p-0={group.collapsible} class:folded={!open}>
							<h3 class="h6 fw-bold m-0 flex-grow-1">
								{#if group.collapsible}
									{@const changedHere = rows.filter((r) => store.isChanged(r.id)).length}
									<button
										type="button"
										class="card-toggle"
										aria-expanded={open}
										aria-controls="group-body-{group.id}"
										onclick={() => (cardOpen[group.id] = !open)}
									>
										<i class="bi" class:bi-chevron-down={open} class:bi-chevron-right={!open}></i>
										<span>{group.title}</span>
										{#if !open && changedHere}
											<span class="changed-count">{changedHere} changed</span>
										{/if}
									</button>
								{:else}
									{group.title}
								{/if}
							</h3>
							{#if store.groupPresets[group.id]}
								<!-- In the header, so the card's preset can be seen and changed while it is folded. -->
								<div class="card-presets">
									<PresetManager
										{store}
										{group}
										kind={group.id}
										storageKey="alttp-maprando:presets:{group.id}"
										selectId="preset-{group.id}"
									/>
								</div>
							{/if}
							{#if store.groupChanged(group)}
								<button
									type="button"
									class="btn btn-sm btn-outline-secondary card-reset"
									disabled={store.locked}
									use:tooltip={`Put everything in ${group.title} back to "${store.listFor(group).selected.name}". The other sections are not touched.`}
									onclick={() => store.resetGroup(group)}
								>
									<i class="bi bi-arrow-counterclockwise"></i> Reset
								</button>
							{/if}
						</header>
						{#if open}
							{@const blocks = blocksOf(group.id, rows)}
							<div class="card-body py-1" id="group-body-{group.id}">
								{#if pool}
									<div class="pool-bar">
										<span>The greyed-out items fill the slots your choices leave over, shared out at random.</span>
										<button
											type="button"
											class="btn btn-sm btn-outline-secondary"
											disabled={store.locked || pool.free < 0}
											onclick={() => store.reroll(group)}
										>
											<i class="bi bi-dice-5"></i> Reroll
										</button>
									</div>
								{/if}
								{#each blocks as block, bi (block.key)}
									<!-- Show only the heading levels that changed since the previous block. -->
									{@const prev = blocks[bi - 1]?.path ?? []}
									{@const from = block.path.findIndex((part, i) => part !== prev[i])}
									{#if from >= 0}
										{#each block.path.slice(from) as part, j (j)}
											{#if from + j === 0}
												<h4 class="subhead">{part}</h4>
											{:else}
												<h5 class="subsubhead">{part}</h5>
											{/if}
										{/each}
									{/if}
									<div class="block" class:grid={block.layout === 'grid'} class:compact={block.layout === 'compact'}>
										{#each block.rows as s (s.id)}
											<SettingRow setting={s} {store} compact={block.layout !== 'list'} />
										{/each}
									</div>
								{/each}
							</div>
						{/if}
					</section>
				{:else}
					<p class="text-body-secondary">No settings match “{search.query}”.</p>
				{/each}
			</CollapsibleSection>

			<CollapsibleSection id="customization" title="Customization" bind:open={customizationOpen}>
				<CustomizationEditor store={custom} />
			</CollapsibleSection>
		</div>

		<!-- Shared by both sections: stays in view while you scroll through either. -->
		<div class="side">
			<SummaryPanel
				{store}
				{generated}
				{generating}
				{error}
				{seed}
				{shareHash}
				ongenerate={generate}
				onnew={newGame}
				onjump={(id) => jumpToSetting('generate', id)}
			/>
			<CustomizationPanel
				store={custom}
				{generated}
				{seed}
				version={gameVersion}
				onjump={(id) => jumpToSetting('customization', id)}
			/>
		</div>
	</div>
</main>

<style>
	.searchbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 1rem;
	}

	.search {
		max-width: 26rem;
	}

	.subhead {
		margin: 1rem 0 0;
		padding-top: 0.6rem;
		border-top: 1px solid var(--bs-border-color-translucent);
		font-size: 0.8rem;
		font-weight: 700;
		letter-spacing: 0.04em;
		text-transform: uppercase;
		opacity: 0.65;
	}

	/* One column of label-left rows; capped so the control stays near its label. */
	.block.compact {
		max-width: 32rem;
	}

	.block.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
		column-gap: 1.5rem;
		align-content: start;
	}

	.subsubhead {
		margin: 0.7rem 0 0.1rem;
		font-size: 0.75rem;
		font-weight: 600;
		opacity: 0.6;
	}

	.card-toggle {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		padding: 0.5rem 1rem;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
	}

	/* Negative vertical margin: the button is taller than the title line, so without it the header grows when it appears. */
	.card-reset {
		flex: none;
		margin: -0.25rem 0.75rem -0.25rem 0.25rem;
		padding: 0.1rem 0.5rem;
		font-size: 0.78rem;
	}

	/* Shrinkable rather than capped, so the controls only wrap when the header is actually out of room. */
	.card-presets {
		flex: 0 1 auto;
		padding: 0.3rem 0.75rem 0.3rem 0;
		font-size: 0.9rem;
	}

	.card-header.folded {
		border-bottom: 0;
	}

	.pool-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		padding: 0.5rem 0;
		font-size: 0.8rem;
		opacity: 0.9;
	}

	.changed-count {
		margin-left: auto;
		font-size: 0.8rem;
		font-weight: 600;
	}

	.subhead:first-child {
		margin-top: 0.4rem;
		border-top: 0;
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
		padding-left: 1.4rem;
		font-size: 0.92rem;
	}

	/* Both panels stick together: Your settings on top, Your customization beneath. */
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
	:global(#customization),
	:global(section[id^='group-']),
	:global(section[id^='custom-group-']) {
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