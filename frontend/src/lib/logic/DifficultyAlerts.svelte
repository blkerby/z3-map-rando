<script lang="ts">
	import type { DifficultyFilter } from './difficultyFilter.svelte';

	let { filter }: { filter: DifficultyFilter } = $props();
</script>

{#if filter.error}
	<div class="text-danger mt-2">{filter.error}</div>
{/if}
{#if filter.presets.notices.length}
	<div class="alert alert-info small py-2 px-3 mt-3 d-flex align-items-start" role="status">
		<div class="me-auto">
			<div class="fw-semibold">
				Some saved settings were updated for this version of the randomizer.
			</div>
			{#each filter.presets.notices as group, i (i)}
				<div class="mt-1">{group.source}:</div>
				<ul class="mb-0">
					{#each group.issues as issue, j (j)}
						<li>
							{issue.path.length ? `${filter.presets.label(issue.path)}: ` : ''}{issue.message}
						</li>
					{/each}
				</ul>
			{/each}
		</div>
		<button
			type="button"
			class="btn-close btn-sm"
			aria-label="Dismiss"
			onclick={() => (filter.presets.notices = [])}
		></button>
	</div>
{/if}
