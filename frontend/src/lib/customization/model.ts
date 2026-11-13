import type { SettingsModel } from '$lib/settings/store.svelte';
import type { Tree } from '$lib/settings/tree';

/**
 * Sample cosmetic options, modelled on the Super Metroid Map Rando customization page. Replace with the
 * real ALTTP list. None of these affect the generated map or the game link.
 */
const defaults: Tree = {
	link_sprite: 'link',
	heart_color: 'de3843',
	menu_speed: 'vanilla',
	quick_swap: false,
	controller: {
		custom: false,
		sword: 'b',
		item: 'y',
		dash: 'a',
		map: 'x',
		previous_item: 'l',
		next_item: 'r'
	},
	music: 'vanilla',
	low_health_beep: 'vanilla',
	reduced_flashing: false,
	screen_shake: 'vanilla'
};

export const customizationModel: SettingsModel = {
	rootPresets: [{ id: 'default', label: 'Default', settings: defaults, scopes: {} }],
	scopes: []
};
