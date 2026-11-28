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
		ITEM_LOCATIONS,
		countPoolItems,
		countStartingItems,
		getItemIcon,
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

	const BOTTLE_CONTENTS = [
		{ value: 'none', label: 'None' },
		{ value: 'empty', label: 'Empty' },
		{ value: 'red_potion', label: 'Red Potion' },
		{ value: 'green_potion', label: 'Green Potion' },
		{ value: 'blue_potion', label: 'Blue Potion' },
		{ value: 'fairy', label: 'Fairy' },
		{ value: 'bee', label: 'Bee' },
		{ value: 'gold_bee', label: 'Golden Bee' }
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
		<ChoiceSetting
			path={['keys']}
			label="Keys"
			options={[
				{ value: 'simple', label: 'Simple' },
				{ value: 'random', label: 'Random' }
			]}
		>
			{#snippet help()}
				<p>Each Small Key opens one door in its dungeon. Simple uses fixed pairings based on the vanilla route. Random chooses new pairings during generation.</p>
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
			{#each notables.filter((n) => n.difficulty === difficulty) as n (`${n.room}/${n.id}`)}
				<ToggleSetting
					path={['skill', 'notables', n.room, n.id]}
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
				label={item.label}
				icon={getItemIcon(item.name)}
			/>
		{:else}
			<ChoiceSetting
				path={['starting_state', 'equipment', item.name]}
				label={item.label}
				icon={getItemIcon(item.name)}
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
	<Group heading="Resources" layout="grid" columns={2}>
		<CountSetting
			path={['starting_state', 'resources', 'bomb_capacity']}
			label="Bomb capacity"
			icon={getItemIcon('bomb_capacity_upgrade')}
			min={0}
			max={99}
		/>
		<CountSetting
			path={['starting_state', 'resources', 'bombs']}
			label="Bombs"
			icon={getItemIcon('bomb')}
			min={0}
			max={store.get(['starting_state', 'resources', 'bomb_capacity']) as number}
		/>
		<CountSetting
			path={['starting_state', 'resources', 'arrow_capacity']}
			label="Arrow capacity"
			icon={getItemIcon('arrow_capacity_upgrade')}
			min={0}
			max={99}
		/>
		<CountSetting
			path={['starting_state', 'resources', 'arrows']}
			label="Arrows"
			icon={getItemIcon('arrow')}
			min={0}
			max={store.get(['starting_state', 'resources', 'arrow_capacity']) as number}
		/>
		<CountSetting
			path={['starting_state', 'resources', 'heart_capacity']}
			label="Heart capacity"
			icon={getItemIcon('heart_container')}
			min={1}
			max={20}
		/>
		<CountSetting
			path={['starting_state', 'resources', 'rupees']}
			label="Rupees"
			icon={getItemIcon('one_rupee')}
			min={0}
			max={999}
		/>
		{#each [1, 2, 3, 4] as n (n)}
			<ChoiceSetting
				path={['starting_state', 'resources', `bottle_${n}`]}
				label="Bottle {n}"
				icon={getItemIcon('bottle')}
				options={BOTTLE_CONTENTS}
				display="dropdown"
			/>
		{/each}
	</Group>
</SectionCard>

<SectionCard
	id="item_pool"
	title="Item pool"
	collapsible
	collapsed
	scope={itemPoolScope}
>
	<Group layout="compact">
		<ToggleSetting
			path={['item_pool', 'deduct_starting_items']}
			label="Deduct starting equipment"
		>
			{#snippet help()}
				<p>Remove starting equipment from the counts below. For instance, if Pegasus Boots is a starting item, then this setting causes it to be removed from the item pool, so it won't be placed anywhere.</p>
				<p>Bottles, hearts, rupees, bombs, and arrows are not considered equipment: their quantities in the pool are not affected by this setting.</p>
			{/snippet}
		</ToggleSetting>
	</Group>
	<Group layout="grid">
		<CountSetting
			path={['item_pool', 'bomb_upgrade_size']}
			label="Bomb upgrade size"
			icon={getItemIcon('bomb_capacity_upgrade')}
			min={1}
			max={99}
		/>
		<CountSetting
			path={['item_pool', 'arrow_upgrade_size']}
			label="Arrow upgrade size"
			icon={getItemIcon('arrow_capacity_upgrade')}
			min={1}
			max={99}
		/>
	</Group>
	{#each ITEM_CATEGORIES as category (category.id)}
		<Group heading={category.title} layout="grid">
			{#each poolItems.filter((i) => i.category === category.id) as item (item.name)}
				<CountSetting
					path={['item_pool', 'items', item.name]}
					label={item.label}
					icon={getItemIcon(item.name)}
					notes={deducted[item.name] ? `${deducted[item.name]} starting` : undefined}
					min={item.min}
					max={ITEM_LOCATIONS}
				/>
			{/each}
		</Group>
	{/each}
	<div class="pool-bar" class:text-danger={poolUsed > ITEM_LOCATIONS}>
		{poolUsed} of {ITEM_LOCATIONS} item locations filled{poolUsed < ITEM_LOCATIONS
			? ' (the rest are left empty)'
			: ''}.
	</div>
</SectionCard>

<style>
	.pool-bar {
		padding: 0.5rem 0;
		font-size: 0.8rem;
		opacity: 0.9;
	}
</style>
