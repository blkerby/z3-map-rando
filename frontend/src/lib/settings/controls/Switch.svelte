<script lang="ts">
	let {
		checked,
		label,
		disabled = false,
		onchange
	}: {
		checked: boolean;
		label: string;
		disabled?: boolean;
		onchange: (v: boolean) => void;
	} = $props();
</script>

<div class="form-check form-switch m-0">
	<input
		class="form-check-input"
		type="checkbox"
		role="switch"
		aria-label={label}
		{checked}
		{disabled}
		onchange={(e) => onchange(e.currentTarget.checked)}
	/>
</div>

<style>
	/* Locked: keep the on/off state readable instead of Bootstrap's heavy fade. */
	.form-check-input:disabled {
		opacity: 0.75;
	}
</style>
