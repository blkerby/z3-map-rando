import { MediaQuery } from 'svelte/reactivity';

/**
 * Screens too narrow for the preset dropdowns' full labels ("Hard + 2 tweaks + 1 unsaved change") beside
 * the header titles, where they show short ones ("Hard + 2 + 1") instead. The reset buttons of headers and
 * setting rows are also left out there, for room.
 */
export const narrow = new MediaQuery('max-width: 599.98px');
