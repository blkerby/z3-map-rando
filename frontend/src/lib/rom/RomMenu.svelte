<script lang="ts">
	import type { RomStore } from './store.svelte';

	let { rom }: { rom: RomStore } = $props();
</script>

<!-- Bootstrap's dropdown JS is loaded by the layout; markup follows its standard pattern. -->
<div class="dropdown">
	<button
		type="button"
		class="rom-btn d-flex align-items-center gap-1"
		data-bs-toggle="dropdown"
		aria-expanded="false"
		disabled={rom.status === 'loading'}
	>
		<i class="bi" class:bi-check-circle-fill={rom.ready} class:bi-exclamation-circle={!rom.ready}></i>
		ROM
	</button>

	<ul class="dropdown-menu dropdown-menu-end">
		<li class="px-3 py-1 small text-body-secondary rom-status">
			{#if rom.ready}
				<div class="fw-semibold text-body text-truncate">{rom.name}</div>
				{#if rom.notRemembered}
					<div>Not saved: it will be forgotten when you close this page.</div>
				{:else}
					<div>Saved in this browser.</div>
				{/if}
			{:else}
				No ROM selected.
			{/if}
		</li>
		<li><hr class="dropdown-divider" /></li>
		<li>
			<button type="button" class="dropdown-item" onclick={() => void rom.choose()}>
				<i class="bi bi-upload me-1"></i>
				{rom.ready ? 'Change ROM…' : 'Select ROM…'}
			</button>
		</li>
		{#if rom.ready}
			<li>
				<button type="button" class="dropdown-item" onclick={() => void rom.clear()}>
					<i class="bi bi-trash me-1"></i> Remove ROM
				</button>
			</li>
		{/if}
	</ul>
</div>

<style>
	.rom-btn {
		padding: 0;
		border: 0;
		background: none;
		color: #ffffff;
		font: inherit;
		font-size: 0.95rem;
	}

	.rom-btn:hover {
		text-decoration: underline;
	}

	.rom-status {
		max-width: 16rem;
	}
</style>
