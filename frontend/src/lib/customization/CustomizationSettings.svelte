<script lang="ts">
	import ChoiceSetting from '$lib/settings/ChoiceSetting.svelte';
	import ToggleSetting from '$lib/settings/ToggleSetting.svelte';
	import { BUTTONS, HEART_COLORS } from './options';

	const SOON = 'Help text coming soon.';
</script>

<ChoiceSetting
	path={['link_sprite']}
	label="Link sprite"
	help="Which character sprite you play as."
	options={[
		{ value: 'link', label: 'Link' },
		{ value: 'darklink', label: 'Dark Link' },
		{ value: 'zelda', label: 'Zelda' },
		{ value: 'bunny', label: 'Bunny' },
		{ value: 'moblin', label: 'Moblin' },
		{ value: 'octorok', label: 'Octorok' },
		{ value: 'random', label: 'Random', desc: 'A different sprite each time you play.' }
	]}
/>
<ChoiceSetting path={['heart_color']} label="Heart color" help={SOON} display="swatches" options={HEART_COLORS} />
<ChoiceSetting
	path={['menu_speed']}
	label="Menu speed"
	help={SOON}
	options={[
		{ value: 'vanilla', label: 'Vanilla', desc: 'Original menu animations.' },
		{ value: 'fast', label: 'Fast', desc: 'Menus open and close quickly.' },
		{ value: 'instant', label: 'Instant', desc: 'No menu animations.' }
	]}
/>
<ToggleSetting path={['quick_swap']} label="Quick swap" help="Cycle through your items with the shoulder buttons without opening the menu." />
<ToggleSetting path={['controller', 'custom']} label="Custom controller layout" help="Remap which button does what.">
	<ChoiceSetting path={['controller', 'sword']} label="Sword" help={SOON} options={BUTTONS} />
	<ChoiceSetting path={['controller', 'item']} label="Use item" help={SOON} options={BUTTONS} />
	<ChoiceSetting path={['controller', 'dash']} label="Dash" help={SOON} options={BUTTONS} />
	<ChoiceSetting path={['controller', 'map']} label="Open map" help={SOON} options={BUTTONS} />
	<ChoiceSetting path={['controller', 'previous_item']} label="Previous item" help={SOON} options={BUTTONS} />
	<ChoiceSetting path={['controller', 'next_item']} label="Next item" help={SOON} options={BUTTONS} />
</ToggleSetting>
<ChoiceSetting
	path={['music']}
	label="Music"
	help={SOON}
	options={[
		{ value: 'vanilla', label: 'Vanilla', desc: 'Original soundtrack.' },
		{ value: 'disabled', label: 'Disabled', desc: 'Sound effects only.' }
	]}
/>
<ChoiceSetting
	path={['low_health_beep']}
	label="Low-health beep"
	help={SOON}
	options={[
		{ value: 'vanilla', label: 'Vanilla' },
		{ value: 'slow', label: 'Slow', desc: 'Beeps half as often.' },
		{ value: 'off', label: 'Off', desc: 'No beeping.' }
	]}
/>
<ToggleSetting
	path={['reduced_flashing']}
	label="Reduced flashing"
	help="Tones down bright screen flashes (lightning, boss hits, and similar)."
/>
<ChoiceSetting
	path={['screen_shake']}
	label="Screen shake"
	help={SOON}
	options={[
		{ value: 'vanilla', label: 'Vanilla' },
		{ value: 'reduced', label: 'Reduced' }
	]}
/>
