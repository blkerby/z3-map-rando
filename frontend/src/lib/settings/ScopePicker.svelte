<script lang="ts">
	import { tooltip } from '$lib/tooltip';
	import type { Scope, SettingsStore } from './store.svelte';

	// The preset dropdown of a child scope (e.g. Skill assumptions). It takes what the root preset chooses
	// ("Hard + 2 tweaks"), unless a preset is picked by hand, which is an unsaved change.
	let { store, scope, selectId }: { store: SettingsStore; scope: Scope; selectId: string } = $props();

	const picked = $derived(store.isScopePicked(scope));
	const tweaks = $derived(store.scopeTweakCount(scope));
	/** '' selects the "+ tweaks" entry. */
	const shown = $derived(picked || !tweaks ? store.selectedScopePreset(scope).id : '');
</script>

<select
	id={selectId}
	aria-label="{scope.label} preset"
	class="form-select form-select-sm w-auto"
	class:changed={picked}
	class:tweaked={store.isScopeTweaked(scope)}
	use:tooltip={picked ? `Picked by hand. Reset goes back to “${store.inheritedScopeLabel(scope)}”.` : undefined}
	disabled={store.locked}
	value={shown}
	onchange={(e) => store.pickScopePreset(scope, e.currentTarget.value)}
>
	{#if store.inheritedTweakCount(scope)}
		<option value="">{store.inheritedScopeLabel(scope)}</option>
	{/if}
	{#each scope.presets as p (p.id)}
		<option value={p.id}>{p.label}</option>
	{/each}
</select>

<style>
	select.changed {
		background-color: rgba(var(--bs-success-rgb), 0.25);
	}

	select.tweaked {
		background-color: rgba(var(--bs-info-rgb), 0.15);
	}
</style>
