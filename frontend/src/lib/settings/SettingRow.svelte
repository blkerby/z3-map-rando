<script lang="ts">
	import { tooltip } from '$lib/tooltip';
	import SettingRow from './SettingRow.svelte';
	import Segmented from './controls/Segmented.svelte';
	import Cards from './controls/Cards.svelte';
	import Swatches from './controls/Swatches.svelte';
	import Switch from './controls/Switch.svelte';
	import Slider from './controls/Slider.svelte';
	import Level from './controls/Level.svelte';
	import NumberInput from './controls/NumberInput.svelte';
	import { flatten, type SettingsStore } from './store.svelte';
	import type { ChoiceSetting, Setting } from './types';

	let {
		setting,
		store,
		nested = false,
		compact = false
	}: { setting: Setting; store: SettingsStore; nested?: boolean; compact?: boolean } = $props();

	let open = $state(false);

	const locked = $derived(store.locked);
	const value = $derived(store.get(setting.id));
	const changed = $derived(store.isChanged(setting.id));
	// Details can be gated on the parent's value (e.g. controller mapping only when "Custom").
	const detailsVisible = $derived(
		!('detailsWhen' in setting) || setting.detailsWhen === undefined || value === setting.detailsWhen
	);
	const details = $derived(detailsVisible && 'details' in setting ? (setting.details ?? []) : []);
	const changedDetails = $derived(flatten(details).filter((d) => store.isChanged(d.id)));

	// Two-way choices (yes/no) render as a switch too, so their label can flip them.
	const isSwitch = $derived(
		setting.kind === 'toggle' ||
			(setting.kind === 'choice' &&
				setting.options.length === 2 &&
				setting.options.every((o) => o.value === 'yes' || o.value === 'no'))
	);
	const switchId = $derived(`${setting.id}-switch`);

	function choiceDisplay(s: ChoiceSetting): 'cards' | 'segmented' | 'dropdown' | 'swatches' {
		if (s.display && s.display !== 'auto') return s.display;
		return s.options.length > 5 ? 'dropdown' : 'segmented';
	}

	const selectedDesc = $derived(
		setting.kind === 'choice' ? setting.options.find((o) => o.value === value)?.desc : undefined
	);

	// Auto-open the details if the user is searching for something inside them.
	$effect(() => {
		if (store.query.trim() && details.some((d) => store.matches(d))) open = true;
	});
</script>

<div class="setting-row" id={setting.id} class:nested class:changed class:compact>
	<div class="left">
	<div class="label-col">
		{#if setting.help}
			<i class="bi bi-info-circle help" use:tooltip={setting.help} aria-label={setting.help}></i>
		{/if}
		<label class="label" class:clickable={isSwitch && !locked} for={isSwitch ? switchId : undefined}>{setting.label}</label>
		{#if setting.badge && !compact}<span class="badge-tag">{setting.badge}</span>{/if}
		{#if changed && !locked}
			<button
				type="button"
				class="btn btn-link btn-sm p-0 reset"
				use:tooltip={`Reset to ${store.baseline[setting.id]}`}
				aria-label="Reset {setting.label}"
				onclick={() => store.reset(setting.id)}
			>
				<i class="bi bi-arrow-counterclockwise"></i>
			</button>
		{/if}
	</div>
	<!-- In compact rows the name needs the room, so the tag drops to a small second line. -->
	{#if setting.badge && compact}<div class="badge-sub">{setting.badge}</div>{/if}
	{#if setting.notes}<div class="notes">{setting.notes}</div>{/if}
	</div>

	<div class="control-col">
		{#if setting.kind === 'choice'}
			{@const mode = choiceDisplay(setting)}
			{#if isSwitch}
				<Switch
					id={switchId}
					checked={value === 'yes'}
					label={setting.label}
					disabled={locked}
					onchange={(v) => store.set(setting.id, v ? 'yes' : 'no')}
				/>
			{:else if mode === 'cards'}
				<Cards
					options={setting.options}
					value={value as string}
					label={setting.label}
					disabled={locked}
					onchange={(v) => store.set(setting.id, v)}
				/>
			{:else if mode === 'swatches'}
				<Swatches
					options={setting.options}
					value={value as string}
					label={setting.label}
					disabled={locked}
					columns={Math.min(12, setting.options.length)}
					onchange={(v) => store.set(setting.id, v)}
				/>
			{:else if mode === 'dropdown'}
				<select
					class="form-select form-select-sm w-auto"
					disabled={locked}
					value={value as string}
					onchange={(e) => store.set(setting.id, e.currentTarget.value)}
				>
					{#each setting.options as o (o.value)}
						<option value={o.value}>{o.label}</option>
					{/each}
				</select>
			{:else}
				<Segmented
					options={setting.options}
					value={value as string}
					label={setting.label}
					disabled={locked}
					onchange={(v) => store.set(setting.id, v)}
				/>
			{/if}
			{#if selectedDesc && mode !== 'cards'}
				<div class="desc">{selectedDesc}</div>
			{/if}
		{:else if setting.kind === 'toggle'}
			<Switch
				id={switchId}
				checked={value as boolean}
				label={setting.label}
				disabled={locked}
				onchange={(v) => store.set(setting.id, v)}
			/>
		{:else if setting.kind === 'calculated'}
			<input
				type="number"
				class="form-control form-control-sm num calc"
				aria-label="{setting.label} (estimated)"
				value={Number(value)}
				disabled
				use:tooltip={'Worked out for you: the slots your other counts leave over are shared out at random between these items. Use Reroll at the top of the Items card to draw again.'}
			/>
		{:else if setting.kind === 'level'}
			<Level
				levels={setting.levels}
				value={value as number}
				label={setting.label}
				disabled={locked}
				onchange={(v) => store.set(setting.id, v)}
			/>
		{:else if setting.display === 'input' && setting.min === 0 && setting.max === 1 && setting.step === 1}
			<!-- Only ever 0 or 1: a No/Yes toggle reads better than a number box. -->
			<Segmented
				options={[
					{ value: '0', label: 'No' },
					{ value: '1', label: 'Yes' }
				]}
				value={String(value)}
				label={setting.label}
				disabled={locked}
				onchange={(v) => store.set(setting.id, Number(v))}
			/>
		{:else if setting.display === 'input'}
			<NumberInput
				value={value as number}
				min={setting.min}
				max={store.limitOf(setting)}
				step={setting.step}
				label={setting.label}
				disabled={locked}
				onchange={(v) => store.set(setting.id, v)}
			/>
		{:else}
			<Slider
				value={value as number}
				min={setting.min}
				max={setting.max}
				step={setting.step}
				unit={setting.unit}
				label={setting.label}
				disabled={locked}
				onchange={(v) => store.set(setting.id, v)}
			/>
		{/if}

		{#if details.length}
			<button
				type="button"
				class="details-toggle"
				aria-expanded={open}
				onclick={() => (open = !open)}
			>
				<i class="bi" class:bi-chevron-right={!open} class:bi-chevron-down={open}></i>
				<span>Customize</span>
				{#if !open}
					<span class="summary">
						{#if changedDetails.length}
							{changedDetails.map((d) => `${d.label}: ${store.display(d.id)}`).join(' · ')}
						{:else}
							all defaults
						{/if}
					</span>
				{/if}
			</button>
			{#if open}
				<div class="details">
					{#each details as d (d.id)}
						<SettingRow setting={d} {store} nested />
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</div>

<style>
	/* Label on top, control underneath, full width, at every nesting level. */
	.setting-row {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 0.4rem;
		/* Left padding gives up the 3px the accent bar takes. */
		padding: 0.85rem 0.5rem 0.85rem calc(0.5rem - 3px);
		/* Bleed out by the padding so the tint doesn't hug the text and the layout doesn't shift. */
		margin-inline: -0.5rem;
		border-left: 3px solid transparent;
		border-radius: var(--bs-border-radius);
	}

	.setting-row.changed {
		background: rgba(var(--bs-success-rgb), 0.1);
		border-left-color: var(--bs-success);
	}

	.setting-row.nested {
		gap: 0.25rem;
		padding-block: 0.55rem;
	}

	/* Hairline between top-level rows inside a group card. */
	:global(.setting-row:not(.nested) + .setting-row:not(.nested):not(.compact)) {
		border-top: 1px solid var(--bs-border-color-translucent);
	}

	/* Compact: label on the left, control on the right, for the grid and compact layouts. */
	.setting-row.compact {
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.5rem;
		padding-block: 0.3rem;
	}

	.compact .label-col {
		min-width: 0;
	}

	/* Never cut a name off: long ones wrap (or, for one unbroken word, break) inside their cell. */
	.compact .label {
		overflow-wrap: anywhere;
	}

	.label-col {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.label {
		font-weight: 500;
		cursor: default;
	}

	.label.clickable {
		cursor: pointer;
	}

	.left {
		min-width: 0;
	}

	.num {
		width: 4.75rem;
		font-variant-numeric: tabular-nums;
	}
	/* Greyed out, no spinner arrows: shows the estimate, can't be edited. */
	.calc {
		appearance: textfield;
		-moz-appearance: textfield;
		cursor: not-allowed;
	}
	.calc::-webkit-inner-spin-button,
	.calc::-webkit-outer-spin-button {
		-webkit-appearance: none;
		margin: 0;
	}

	.notes {
		font-size: 0.78rem;
		line-height: 1.25;
		opacity: 0.7;
	}

	.badge-sub {
		font-size: 0.7rem;
		line-height: 1.1;
		opacity: 0.65;
	}

	.badge-tag {
		white-space: nowrap;
		padding: 0 0.45rem;
		border: 1px solid var(--bs-border-color);
		border-radius: 999px;
		font-size: 0.7rem;
		opacity: 0.7;
	}

	.help {
		opacity: 0.5;
		font-size: 0.85rem;
		cursor: help;
	}

	.reset {
		margin-left: 0.25rem;
		text-decoration: none;
	}

	.control-col {
		min-width: 0;
	}

	.desc {
		margin-top: 0.3rem;
		font-size: 0.82rem;
		opacity: 0.7;
	}

	.details-toggle {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		margin-top: 0.4rem;
		padding: 0;
		border: 0;
		background: none;
		color: var(--bs-link-color);
		font-size: 0.85rem;
		text-align: left;
	}

	.summary {
		color: var(--bs-body-color);
		opacity: 0.6;
	}

	.details {
		margin-top: 0.4rem;
		padding-left: 0.9rem;
		border-left: 2px solid var(--bs-border-color);
	}
</style>
