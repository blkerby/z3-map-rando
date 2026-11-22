<script lang="ts">
	import type { Scope, SettingsStore } from './store.svelte';

	// The preset dropdown of a child scope (e.g. Skill assumptions). It takes what the root preset chooses
	// ("Hard + 2 tweaks"), unless a preset is picked by hand, which is an unsaved change.
	let { store, scope, selectId }: { store: SettingsStore; scope: Scope; selectId: string } = $props();

	const picked = $derived(store.isScopePicked(scope));
	const tweaks = $derived(store.scopeTweakCount(scope));
	/** Unsaved changes inside the scope, besides its own pick (which the dropdown already names). */
	const changeCount = $derived(store.countScopeChanges(scope));
	/** What the dropdown shows while there are unsaved changes: "Hard + 2 tweaks + 1 unsaved change". */
	const label = $derived(`${store.scopeLabel(scope)} + ${changeCount} unsaved change${changeCount === 1 ? '' : 's'}`);
	/** Value of the hidden entry that shows `label` (not a preset id). */
	const CURRENT = '*current';
	/**
	 * What the dropdown shows: the entry for the unsaved changes, else the preset picked by hand, else ''
	 * for the "+ tweaks" entry, else the preset chosen for it.
	 */
	const shown = $derived(changeCount ? CURRENT : picked || !tweaks ? store.selectedScopePreset(scope).id : '');
</script>

<select
	id={selectId}
	aria-label="{scope.label} preset"
	class="form-select form-select-sm w-auto"
	class:changed={picked || changeCount > 0}
	class:tweaked={!changeCount && store.isScopeTweaked(scope)}
	title={picked ? `Picked by hand. Reset goes back to “${store.inheritedScopeLabel(scope)}”.` : undefined}
	data-lockable
	disabled={store.locked}
	value={shown}
	onchange={(e) => store.pickScopePreset(scope, e.currentTarget.value)}
>
	{#if changeCount}
		<option value={CURRENT} hidden disabled>{label}</option>
	{/if}
	{#if store.inheritedTweakCount(scope)}
		<option value="">{store.inheritedScopeLabel(scope)}</option>
	{/if}
	{#each scope.presets as p (p.id)}
		<option value={p.id}>{p.label}</option>
	{/each}
</select>

<style>
	select {
		max-width: 100%;
		text-overflow: ellipsis;
	}

	select.changed {
		background-color: rgba(var(--bs-success-rgb), 0.15);
	}

	select.tweaked {
		background-color: rgba(var(--bs-info-rgb), 0.15);
	}
</style>
