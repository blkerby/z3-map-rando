<script lang="ts">
	import ChoiceSetting from '$lib/settings/ChoiceSetting.svelte';
	import { HEART_COLORS } from './options';
</script>

<ChoiceSetting path={['heart_color']} label="Heart color" help="Color of the hearts in the life meter." display="swatches" options={HEART_COLORS} />
