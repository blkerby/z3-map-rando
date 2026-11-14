<script lang="ts">
	import ChoiceSetting from '$lib/settings/ChoiceSetting.svelte';
	import CountSetting from '$lib/settings/CountSetting.svelte';
	import Group from '$lib/settings/Group.svelte';
	import LevelSetting from '$lib/settings/LevelSetting.svelte';
	import SectionCard from '$lib/settings/SectionCard.svelte';
	import ToggleSetting from '$lib/settings/ToggleSetting.svelte';
	import { getStore } from '$lib/settings/context';
	import { DIFFICULTIES, ITEM_LOCATIONS, countPoolItems, notables, poolItems, proficiencies, skillScope, tech } from './data';

	const store = getStore();

	const ITEM_CATEGORIES = [
		{ id: 'equipment', title: 'Equipment' },
		{ id: 'upgrades', title: 'Upgrades' },
		{ id: 'ammo', title: 'Ammo' },
		{ id: 'rupees', title: 'Rupees' },
		{ id: 'hearts', title: 'Hearts' }
	];

	const ITEM_LABELS: Record<string, string> = {
		fire_rod: 'Fire Rod',
		ice_rod: 'Ice Rod',
		hammer: 'Hammer',
		hookshot: 'Hookshot',
		bow: 'Bow',
		boomerang: 'Boomerang',
		magic_powder: 'Magic Powder',
		bombos: 'Bombos',
		ether: 'Ether',
		quake: 'Quake',
		lamp: 'Lamp',
		shovel: 'Shovel',
		cane_of_somaria: 'Cane of Somaria',
		cane_of_byrna: 'Cane of Byrna',
		cape: 'Magic Cape',
		magic_mirror: 'Magic Mirror',
		book_of_mudora: 'Book of Mudora',
		flippers: 'Flippers',
		moon_pearl: 'Moon Pearl',
		bug_catching_net: 'Bug-Catching Net',
		mushroom: 'Mushroom',
		bottle: 'Bottle',
		pegasus_boots: 'Pegasus Boots',
		ocarina: 'Ocarina',
		sword: 'Sword',
		shield: 'Shield',
		glove: 'Glove',
		mail: 'Mail',
		half_magic: 'Half Magic',
		silver_arrows: 'Silver Arrows',
		arrow: 'Arrow (1)',
		three_bombs: 'Bombs (3)',
		ten_arrows: 'Arrows (10)',
		one_rupee: '1 Rupee',
		five_rupees: '5 Rupees',
		twenty_rupees: '20 Rupees',
		fifty_rupees: '50 Rupees',
		one_hundred_rupees: '100 Rupees',
		three_hundred_rupees: '300 Rupees',
		heart_container: 'Heart Container',
		piece_of_heart: 'Piece of Heart'
	};

	const poolUsed = $derived(countPoolItems(store));
</script>

<SectionCard id="skill" title="Skill assumptions" icon="speedometer2" collapsible collapsed scope={skillScope}>
	<Group heading="Proficiencies">
		{#each proficiencies as p (p.id)}
			<LevelSetting
				path={['skill', 'proficiencies', p.id]}
				label={p.name}
				levels={p.levels.map((l) => ({ value: l.level, label: l.difficulty }))}
			/>
		{/each}
	</Group>
	<!-- Techniques and notables by difficulty. Unranked ones (e.g. "Ignored") are not shown and keep their default. -->
	{#each DIFFICULTIES as difficulty (difficulty)}
		<Group heading={difficulty} layout="compact">
			{#each tech.filter((t) => t.difficulty === difficulty) as t (t.id)}
				<ToggleSetting path={['skill', 'tech', t.id]} label={t.name} />
			{/each}
			{#each notables.filter((n) => n.difficulty === difficulty) as n (`${n.world}/${n.room}/${n.id}`)}
				<ToggleSetting path={['skill', 'notables', n.world, n.room, n.id]} label="{n.roomName}: {n.name}" />
			{/each}
		</Group>
	{/each}
</SectionCard>

<SectionCard id="general" title="General" icon="map">
	<ChoiceSetting
		path={['map_layout']}
		label="Map layout"
		display="cards"
		options={[
			{ value: 'random', label: 'Randomized', desc: 'The map is shuffled.' },
			{ value: 'vanilla', label: 'Vanilla', desc: 'The original map, unchanged.' }
		]}
	/>
</SectionCard>

<SectionCard id="item_pool" title="Item pool" icon="box-seam" collapsible collapsed>
	{#snippet header()}
		<div class="pool-bar" class:text-danger={poolUsed > ITEM_LOCATIONS}>
			{poolUsed} of {ITEM_LOCATIONS} item locations filled{poolUsed < ITEM_LOCATIONS ? ' (the rest are left empty)' : ''}.
		</div>
	{/snippet}
	{#each ITEM_CATEGORIES as category (category.id)}
		<Group heading={category.title} layout="grid">
			{#each poolItems.filter((i) => i.category === category.id) as item (item.name)}
				<CountSetting path={['item_pool', item.name]} label={ITEM_LABELS[item.name] ?? item.name} min={item.min} max={ITEM_LOCATIONS} />
			{/each}
		</Group>
	{/each}
</SectionCard>

<style>
	.pool-bar {
		padding: 0.5rem 0;
		font-size: 0.8rem;
		opacity: 0.9;
	}
</style>
