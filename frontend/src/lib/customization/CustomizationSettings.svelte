<script lang="ts">
	import ChoiceSetting from '$lib/settings/ChoiceSetting.svelte';
	import ToggleSetting from '$lib/settings/ToggleSetting.svelte';
	import { HEART_COLORS } from './options';
</script>

<ChoiceSetting path={['heart_color']} label="Heart color" display="swatches" options={HEART_COLORS}>
	{#snippet help()}
		<p>Color of the hearts in the life meter.</p>
	{/snippet}
</ChoiceSetting>
<ToggleSetting path={['reduced_flashing']} label="Reduced flashing">
	{#snippet help()}
		<p>Suppress weather, spell, electrical, cutscene, and boss flashes while preserving Ether's invisible-floor reveal.</p>
	{/snippet}
</ToggleSetting>
