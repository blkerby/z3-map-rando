<script lang="ts">
	import type { RomStore } from './store.svelte';

	let { rom }: { rom: RomStore } = $props();

	let dialog = $state<HTMLDialogElement>();
	let dragging = $state(false);

	// Keep the native <dialog> in step with the store.
	$effect(() => {
		if (!dialog) return;
		if (rom.dialogOpen && !dialog.open) dialog.showModal();
		else if (!rom.dialogOpen && dialog.open) dialog.close();
	});

	function pick(files: FileList | null | undefined) {
		const file = files?.[0];
		if (file) void rom.select(file);
	}

	function ondrop(e: DragEvent) {
		e.preventDefault();
		dragging = false;
		pick(e.dataTransfer?.files);
	}
</script>

<!-- Escape fires `cancel`; clicking the backdrop is deliberately ignored so a stray click can't dismiss it. -->
<dialog
	bind:this={dialog}
	class="rom-dialog"
	aria-labelledby="rom-title"
	oncancel={(e) => {
		e.preventDefault();
		rom.cancel();
	}}
>
	<div class="p-4">
		<h2 id="rom-title" class="h5 fw-bold mb-2">{rom.ready ? 'Change your ROM' : 'Select your ROM'}</h2>
		<p class="small mb-3">
			Generating a game needs your own copy of <b>A Link to the Past</b> (<em>Zelda no Densetsu (V1.0) (J).sfc</em>). The file stays in this
			browser; it is never uploaded.
		</p>
		{#if rom.ready}
			<p class="small text-body-secondary mb-3">
				Current ROM: <strong>{rom.name}</strong>. It stays in use unless you pick a different valid file.
			</p>
		{/if}

		<label
			class="drop"
			class:dragging
			ondragover={(e) => {
				e.preventDefault();
				dragging = true;
			}}
			ondragleave={() => (dragging = false)}
			{ondrop}
		>
			<input
				type="file"
				class="visually-hidden"
				accept=".sfc,.smc,.rom,.bin"
				disabled={rom.busy}
				onchange={(e) => {
					pick(e.currentTarget.files);
					e.currentTarget.value = ''; // allow re-picking the same file after an error
				}}
			/>
			{#if rom.busy}
				<span class="spinner-border spinner-border-sm" aria-hidden="true"></span> Checking…
			{:else}
				<i class="bi bi-upload"></i> Choose a ROM file, or drop it here
			{/if}
		</label>

		{#if rom.error}
			<div class="alert alert-danger small py-2 px-3 mt-3 mb-0" role="alert">{rom.error}</div>
		{/if}

		<div class="d-flex justify-content-end mt-3">
			<button type="button" class="btn btn-outline-secondary btn-sm" onclick={() => rom.cancel()}>
				Cancel
			</button>
		</div>
	</div>
</dialog>

<style>
	.rom-dialog {
		width: min(32rem, calc(100vw - 2rem));
		padding: 0;
		border: 1px solid var(--bs-border-color);
		border-radius: 0.6rem;
		background: var(--bs-body-bg);
		color: var(--bs-body-color);
	}

	.rom-dialog::backdrop {
		background: rgb(0 0 0 / 0.55);
	}

	.drop {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 1.75rem 1rem;
		border: 2px dashed var(--bs-border-color);
		border-radius: 0.5rem;
		cursor: pointer;
	}

	.drop:hover,
	.drop.dragging,
	.drop:focus-within {
		border-color: var(--bs-primary);
		background: var(--bs-tertiary-bg);
	}
</style>
