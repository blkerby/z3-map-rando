import { settingsData } from '$lib/generation/data';
import type { RootPreset, SettingsModel } from '$lib/settings/store.svelte';

// Cosmetic options. None of these affect the generated map or the game link.
const rootPresets: RootPreset[] = [];
for (const p of settingsData.presets.customization) rootPresets.push({ ...p, scopes: {} });

export const customizationModel: SettingsModel = { rootPresets, scopes: [] };
