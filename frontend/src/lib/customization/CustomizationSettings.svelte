<script lang="ts">
	import ChoiceSetting from '$lib/settings/ChoiceSetting.svelte';
	import ToggleSetting from '$lib/settings/ToggleSetting.svelte';
	import { HEART_COLORS } from './options';
</script>

<ChoiceSetting path={['heart_color']} label="Heart color" help="Color of the hearts in the life meter." display="swatches" options={HEART_COLORS} />
<ToggleSetting path={['reduced_flashing']} label="Reduced flashing" help="Disable rain lightning and Ether medallion flashes." />
