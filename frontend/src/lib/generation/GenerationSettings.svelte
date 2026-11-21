<script lang="ts">
	import ChoiceSetting from '$lib/settings/ChoiceSetting.svelte';
	import CountSetting from '$lib/settings/CountSetting.svelte';
	import Group from '$lib/settings/Group.svelte';
	import LevelSetting from '$lib/settings/LevelSetting.svelte';
	import SectionCard from '$lib/settings/SectionCard.svelte';
	import ToggleSetting from '$lib/settings/ToggleSetting.svelte';
	import { getStore } from '$lib/settings/context';
	import {
		DIFFICULTIES,
		ITEM_LABELS,
		ITEM_LOCATIONS,
		countPoolItems,
		countStartingItems,
		itemPoolScope,
		startingEquipment,
		startingStateScope,
		notables,
		poolItems,
		proficiencies,
		skillScope,
		tech
	} from './data';

	const store = getStore();

	const ITEM_CATEGORIES = [
		{ id: 'equipment', title: 'Equipment' },
		{ id: 'progressive', title: 'Progressive equipment' },
		{ id: 'resources', title: 'Resources' },
		{ id: 'consumables', title: 'Consumables' }
	];

	const poolUsed = $derived(countPoolItems(store));
	/** Pool items deducted for the starting state, by item name. */
	const deducted = $derived(
		store.get(['item_pool', 'deduct_starting_items']) ? countStartingItems(store) : {}
	);
</script>

<div class="mb-3">
	<Group layout="compact">
		<ChoiceSetting
			path={['map_layout']}
			label="Map layout"
			options={[
				{ value: 'vanilla', label: 'Vanilla' },
				{ value: 'random', label: 'Randomized' }
			]}
		>
			{#snippet help()}
				<p><strong>Vanilla:</strong> The original overworld map is used, unchanged.</p>
				<p><strong>Randomized:</strong> The overworld areas are shuffled into a new map arrangement.</p>
			{/snippet}
		</ChoiceSetting>
	</Group>
</div>

<SectionCard
	id="skill"
	title="Skill assumptions"
	collapsible
	collapsed
	scope={skillScope}
>
	<Group heading="Proficiencies" layout="compact">
		{#each proficiencies as p (p.id)}
			<LevelSetting
				path={['skill', 'proficiencies', p.id]}
				label={p.name}
				levels={p.levels.map((l) => ({ value: l.level, label: l.level === 0 ? 'Off' : String(l.level) }))}
			>
				{#snippet help()}
					{#each p.levels as l (l.level)}
						{#if l.note !== null}
							<p><strong>Level {l.level}</strong> ({l.difficulty}): {l.note}</p>
						{/if}
					{/each}
				{/snippet}
			</LevelSetting>
		{/each}
	</Group>
	<!-- Techniques and notables by difficulty. Unranked ones (e.g. "Ignored") are not shown and keep their default. -->
	{#each DIFFICULTIES as difficulty (difficulty)}
		<Group heading={difficulty} layout="compact">
			{#each tech.filter((t) => t.difficulty === difficulty) as t (t.id)}
				<ToggleSetting path={['skill', 'tech', t.id]} label={t.name}>
					{#snippet help()}
						<p>{t.note}</p>
					{/snippet}
				</ToggleSetting>
			{/each}
			{#each notables.filter((n) => n.difficulty === difficulty) as n (`${n.world}/${n.room}/${n.id}`)}
				<ToggleSetting
					path={['skill', 'notables', n.world, n.room, n.id]}
					label="{n.roomName}: {n.name}"
				>
					{#snippet help()}
						<p>{n.note}</p>
						{#if n.detailNote !== null}<p>{n.detailNote}</p>{/if}
					{/snippet}
				</ToggleSetting>
			{/each}
		</Group>
	{/each}
</SectionCard>

<!-- Starting equipment settings of one item category. -->
{#snippet equipmentSettings(category: string)}
	{#each startingEquipment.filter((e) => e.item.category === category) as { item, tiers } (item.name)}
		{#if tiers === null}
			<ToggleSetting
				path={['starting_state', 'equipment', item.name]}
				label={ITEM_LABELS[item.name]}
			/>
		{:else}
			<ChoiceSetting
				path={['starting_state', 'equipment', item.name]}
				label={ITEM_LABELS[item.name]}
				options={tiers}
				display="dropdown"
			/>
		{/if}
	{/each}
{/snippet}

<SectionCard
	id="starting_state"
	title="Starting state"
	collapsible
	collapsed
	scope={startingStateScope}
>
	<!-- Grouped and ordered like the item pool below. -->
	<Group heading="Equipment" layout="grid">
		{@render equipmentSettings('equipment')}
	</Group>
	<Group heading="Progressive equipment" layout="grid">
		{@render equipmentSettings('progressive')}
	</Group>
	<Group heading="Resources" layout="grid">
    	<CountSetting
    		path={['starting_state', 'resources', 'heart_capacity']}
    		label="Heart capacity"
    		min={1}
    		max={20}
    	/>
		<CountSetting
			path={['starting_state', 'resources', 'bottles']}
			label="Bottles"
			min={0}
			max={4}
		/>
		<CountSetting
			path={['starting_state', 'resources', 'bomb_capacity']}
			label="Bomb capacity"
			min={0}
			max={99}
		/>
		<CountSetting
			path={['starting_state', 'resources', 'arrow_capacity']}
			label="Arrow capacity"
			min={0}
			max={99}
		/>
	</Group>
	<Group heading="Consumables" layout="grid">
		<CountSetting
			path={['starting_state', 'resources', 'bombs']}
			label="Bombs"
			min={0}
			max={store.get(['starting_state', 'resources', 'bomb_capacity']) as number}
		/>
		<CountSetting
			path={['starting_state', 'resources', 'arrows']}
			label="Arrows"
			min={0}
			max={store.get(['starting_state', 'resources', 'arrow_capacity']) as number}
		/>
		<CountSetting
			path={['starting_state', 'resources', 'rupees']}
			label="Rupees"
			min={0}
			max={999}
		/>
	</Group>
</SectionCard>

<SectionCard
	id="item_pool"
	title="Item pool"
	collapsible
	collapsed
	scope={itemPoolScope}
>
	{#snippet header()}
		<div class="pool-bar" class:text-danger={poolUsed > ITEM_LOCATIONS}>
			{poolUsed} of {ITEM_LOCATIONS} item locations filled{poolUsed < ITEM_LOCATIONS
				? ' (the rest are left empty)'
				: ''}.
		</div>
	{/snippet}
	<Group heading="Pool behavior">
		<ToggleSetting
			path={['item_pool', 'deduct_starting_items']}
			label="Deduct starting equipment"
		>
			{#snippet help()}
				<p>Remove starting equipment from the counts below. Bottles, health, capacities, ammunition, rupees, and flags do not reduce the pool.</p>
			{/snippet}
		</ToggleSetting>
		<CountSetting
			path={['item_pool', 'bomb_upgrade_size']}
			label="Bomb upgrade size"
			min={1}
			max={99}
		>
			{#snippet help()}
				<p>Capacity and ammunition granted by each bomb capacity upgrade.</p>
			{/snippet}
		</CountSetting>
		<CountSetting
			path={['item_pool', 'arrow_upgrade_size']}
			label="Arrow upgrade size"
			min={1}
			max={99}
		>
			{#snippet help()}
				<p>Capacity and ammunition granted by each arrow capacity upgrade.</p>
			{/snippet}
		</CountSetting>
	</Group>
	{#each ITEM_CATEGORIES as category (category.id)}
		<Group heading={category.title} layout="grid">
			{#each poolItems.filter((i) => i.category === category.id) as item (item.name)}
				<CountSetting
					path={['item_pool', 'items', item.name]}
					label={ITEM_LABELS[item.name] ?? item.name}
					notes={deducted[item.name] ? `−${deducted[item.name]} starting` : undefined}
					min={item.min}
					max={ITEM_LOCATIONS}
				/>
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
