<script lang="ts">
	import type { Snippet } from 'svelte';
	import Switch from './controls/Switch.svelte';
	import SettingRow from './SettingRow.svelte';
	import { getStore } from './context';
	import type { Path } from './tree';

	let {
		path,
		label,
		help,
		notes,
		badge,
		children
	}: {
		path: Path;
		label: string;
		help?: string;
		notes?: string;
		badge?: string;
		/** Settings shown underneath while the toggle is on. */
		children?: Snippet;
	} = $props();

	const store = getStore();
	const switchId = $derived(`${store.elementId(path)}-switch`);
	const on = $derived(store.get(path) === true);
</script>

<SettingRow {path} {label} {help} {notes} {badge} labelFor={switchId} format={(v) => (v ? 'On' : 'Off')}>
	{#snippet control()}
		<Switch id={switchId} checked={on} {label} disabled={store.locked} onchange={(v) => store.set(path, v)} />
	{/snippet}
	{#snippet nested()}
		<!-- Kept mounted while off, so the settings stay known to the summary and search. -->
		{#if children}<div hidden={!on}>{@render children()}</div>{/if}
	{/snippet}
</SettingRow>
