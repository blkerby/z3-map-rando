<script lang="ts">
	import { resolve } from '$app/paths';
	import type { Scope, SettingsStore } from './store.svelte';
	import type { Path } from './tree';

	let {
		store,
		generated = false,
		seed = null,
		onreuse,
		onjump,
		onjumpcard,
		onsave
	}: {
		store: SettingsStore;
		/** True once a game exists for these settings (the settings are locked). */
		generated?: boolean;
		seed?: string | null;
		/** Start a new game from these settings. */
		onreuse: () => void;
		/** Scroll to a setting (the page also clears the search and opens its card). */
		onjump: (path: Path) => void;
		/** Scroll to a card. */
		onjumpcard: (id: string) => void;
		/** Save the settings as a preset (see PresetManager's `save`). */
		onsave: () => void;
	} = $props();

	let copied = $state(false);

	const presetName = $derived(store.presetName);
	/** The selected preset, if it is one of the player's own (saving goes into it). */
	const mine = $derived(store.userPreset(store.selectedId));
	const changeCount = $derived(store.changeCount);

	type Group = { scope?: Scope; title: string; paths: Path[]; picked: boolean };

	/**
	 * Settings grouped by scope: the root's own settings (untitled), then each child scope that has settings
	 * here or is in `picked` (its preset chosen by hand).
	 */
	function groupByScope(paths: Path[], picked: Scope[]): Group[] {
		const groups: Group[] = [{ title: '', paths: [], picked: false }];
		for (const scope of store.model.scopes) {
			groups.push({ scope, title: `${scope.label}: ${store.scopeLabel(scope)}`, paths: [], picked: picked.includes(scope) });
		}
		for (const path of paths) {
			const scope = store.scopeOf(path);
			groups[scope ? store.model.scopes.indexOf(scope) + 1 : 0].paths.push(path);
		}
		const out: Group[] = [];
		for (const g of groups) if (g.paths.length || g.picked) out.push(g);
		return out;
	}

	const changeGroups = $derived(groupByScope(store.changedPaths, store.pickedScopes));
	const tweakGroups = $derived(groupByScope(store.tweakedPaths, []));
	const tweakCount = $derived(store.tweakedPaths.length);
	/** Folded or unfolded by hand; otherwise open only while there are no unsaved changes, so those stand out. */
	let tweaksOpenByHand = $state<boolean | null>(null);
	const tweaksOpen = $derived(tweaksOpenByHand ?? changeCount === 0);

	/** The card a scope's preset picker is in. */
	function cardOfScope(scopeId: string): string {
		for (const c of Object.values(store.controls)) if (c.path[0] === scopeId && c.cardId) return c.cardId;
		return scopeId;
	}

	async function copyLink() {
		if (!seed) return;
		await navigator.clipboard.writeText(`${location.origin}${resolve('/seed/[name]', { name: seed })}/`);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<aside id="summary-panel" class="summary card">
	<div class="card-body">
		{#if generated}
			{#if seed}<div class="small mb-2"><strong>Seed</strong> <code class="seed">{seed}</code></div>{/if}
		{:else}
			<h2 class="h6 fw-bold mb-2">Generation settings</h2>
		{/if}

		<div class="small mb-2 text-body-secondary">
			{#if changeCount}
				{changeCount} change{changeCount === 1 ? '' : 's'} from <strong>{presetName}</strong>
			{:else if generated}
				Based on <strong>{presetName}</strong>
			{:else}
				Matches <strong>{presetName}</strong>
			{/if}
		</div>

		{#if changeCount}
			{#each changeGroups as g (g.title)}
				{#if g.title}<div class="tweak-group">{g.title}</div>{/if}
				<div class="chips mb-2">
					{#if g.picked && g.scope}
						{@const scope = g.scope}
						<button type="button" class="chip" onclick={() => onjumpcard(cardOfScope(scope.id))}>
							Preset: <strong>{store.selectedScopePreset(scope).label}</strong>
						</button>
					{/if}
					{#each g.paths as path (path.join('/'))}
						<button type="button" class="chip" onclick={() => onjump(path)}>
							{store.label(path)}: <strong>{store.display(path)}</strong>
						</button>
					{/each}
				</div>
			{/each}
			<button type="button" class="btn btn-link btn-sm p-0 mb-3" onclick={onsave}>
				<!-- No space after the icon, so the underline starts at the text. -->
				<i class="bi bi-floppy me-2"></i>{mine ? `Save to “${mine.name}”` : 'Save as preset…'}
			</button>
		{/if}

		{#if tweakCount}
			<button type="button" class="tweaks-toggle" aria-expanded={tweaksOpen} onclick={() => (tweaksOpenByHand = !tweaksOpen)}>
				<i class="bi" class:bi-chevron-down={tweaksOpen} class:bi-chevron-right={!tweaksOpen}></i>
				{tweakCount} tweak{tweakCount === 1 ? '' : 's'}{#if !generated}{' '}from <strong>{store.basePreset.label}</strong>{/if}
			</button>
			{#if tweaksOpen}
				{#each tweakGroups as g (g.title)}
					{#if g.title}<div class="tweak-group">{g.title}</div>{/if}
					<div class="chips mb-2">
						{#each g.paths as path (path.join('/'))}
							<button type="button" class="chip tweak" onclick={() => onjump(path)}>
								{store.label(path)}: <strong>{store.display(path)}</strong>
							</button>
						{/each}
					</div>
				{/each}
			{/if}
		{/if}

		{#if generated}
			<div class="d-grid gap-2 mt-3">
				<button type="button" class="btn btn-primary" onclick={copyLink}>
					<i class="bi bi-link-45deg"></i>
					{copied ? 'Copied!' : 'Copy game link'}
				</button>
				<button type="button" class="btn btn-outline-secondary btn-sm" onclick={onreuse}>
					<i class="bi bi-arrow-repeat"></i>
					Use these settings again
				</button>
			</div>
		{/if}
	</div>
</aside>

<style>
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}

	.chip {
		border: 1px solid var(--bs-border-color);
		/* Buttons center their text; long chips wrap, so keep the lines left-aligned. */
		border-radius: 0.9rem;
		padding: 0.1rem 0.6rem;
		background: var(--bs-body-bg);
		color: var(--bs-body-color);
		font-size: 0.78rem;
		text-align: left;
	}

	.chip:hover {
		border-color: var(--bs-primary);
	}

	/* Fainter than a change, like the tweaked rows. */
	.chip.tweak {
		opacity: 0.8;
		border-style: dashed;
	}

	.tweaks-toggle {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		margin-bottom: 0.4rem;
		padding: 0;
		border: 0;
		background: none;
		color: var(--bs-secondary-color);
		font-size: 0.875rem;
		text-align: left;
	}

	.tweak-group {
		margin-bottom: 0.25rem;
		font-size: 0.78rem;
		color: var(--bs-secondary-color);
	}

	.seed {
		font-size: 0.9rem;
		letter-spacing: 0.05em;
		user-select: all;
	}
</style>
