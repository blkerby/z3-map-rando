import defaultPreset from '$data/presets/customization/root/default.json';
import type { SettingsModel } from '$lib/settings/store.svelte';
import type { Tree } from '$lib/settings/tree';

// Sample cosmetic options, modelled on the Super Metroid Map Rando customization page. Replace with the real
// ALTTP list. None of these affect the generated map or the game link.
export const customizationModel: SettingsModel = {
	rootPresets: [{ id: defaultPreset.id, label: defaultPreset.label, settings: defaultPreset.settings as Tree, scopes: {} }],
	scopes: []
};
