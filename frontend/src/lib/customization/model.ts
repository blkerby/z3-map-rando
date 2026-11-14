import { settingsData } from '$lib/generation/data';
import type { RootPreset, SettingsModel } from '$lib/settings/store.svelte';

// Sample cosmetic options, modelled on the Super Metroid Map Rando customization page. Replace with the real
// ALTTP list. None of these affect the generated map or the game link.
const rootPresets: RootPreset[] = [];
for (const p of settingsData.customization) rootPresets.push({ ...p, scopes: {} });

export const customizationModel: SettingsModel = { rootPresets, scopes: [] };
