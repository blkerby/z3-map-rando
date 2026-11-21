<script lang="ts">
	import ChoiceSetting from '$lib/settings/ChoiceSetting.svelte';
	import Group from '$lib/settings/Group.svelte';
	import ToggleSetting from '$lib/settings/ToggleSetting.svelte';
	import { HEART_COLORS } from './options';
</script>

<Group layout="compact">
	<ChoiceSetting path={['heart_color']} label="Heart color" display="swatches" options={HEART_COLORS}>
		{#snippet help()}
			<p>Color of hearts in Link's life meter.</p>
		{/snippet}
	</ChoiceSetting>
	<ToggleSetting path={['reduced_flashing']} label="Reduced flashing">
		{#snippet help()}
			<p>Remove the game's whole-screen bright flashes, including from lightning, cutscenes, and bosses.</p>
		{/snippet}
	</ToggleSetting>
</Group>
