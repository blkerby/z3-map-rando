import type { ChoiceOption, Item, Preset, Setting } from './types';

/**
 * Sample cosmetic options, modelled on the Super Metroid Map Rando customization page.
 * Replace with the real ALTTP list. None of these affect the generated map or the game link.
 * Ids must not collide with ids in schema.ts (they are used as DOM ids).
 */

const BUTTONS: ChoiceOption[] = ['A', 'B', 'X', 'Y', 'L', 'R'].map((b) => ({
	value: b.toLowerCase(),
	label: b
}));

function button(id: string, label: string, def: string, help?: string): Setting {
	return { id, label, help, kind: 'choice', options: BUTTONS, default: def };
}

/** The customization settings, loose (not grouped into sections). */
export const customizationItems: Item[] = [
	{
		id: 'linkSprite',
		label: 'Link sprite',
		kind: 'choice',
		default: 'link',
		help: 'Which character sprite you play as.',
		options: [
			{ value: 'link', label: 'Link' },
			{ value: 'darklink', label: 'Dark Link' },
			{ value: 'zelda', label: 'Zelda' },
			{ value: 'bunny', label: 'Bunny' },
			{ value: 'moblin', label: 'Moblin' },
			{ value: 'octorok', label: 'Octorok' },
			{ value: 'random', label: 'Random', desc: 'A different sprite each time you play.' }
		]
	},
	{
		id: 'heartColor',
		label: 'Heart color',
		help: 'Help text coming soon.',
		kind: 'choice',
		display: 'swatches',
		default: 'de3843',
		options: [
			{ value: 'de3894', label: 'Pink', swatch: '#de3894' },
			{ value: 'de3843', label: 'Red', swatch: '#de3843' },
			{ value: 'de8038', label: 'Orange', swatch: '#de8038' },
			{ value: 'ded338', label: 'Yellow', swatch: '#ded338' },
			{ value: '96de38', label: 'Lime', swatch: '#96de38' },
			{ value: '43de38', label: 'Green', swatch: '#43de38' },
			{ value: '38de80', label: 'Spring green', swatch: '#38de80' },
			{ value: '38ded3', label: 'Cyan', swatch: '#38ded3' },
			{ value: '3896de', label: 'Sky blue', swatch: '#3896de' },
			{ value: '3843de', label: 'Blue', swatch: '#3843de' },
			{ value: '8038de', label: 'Violet', swatch: '#8038de' },
			{ value: 'd338de', label: 'Magenta', swatch: '#d338de' },
			{ value: 'de8cba', label: 'Light pink', swatch: '#de8cba' },
			{ value: 'de8c91', label: 'Light red', swatch: '#de8c91' },
			{ value: 'deaf8c', label: 'Light orange', swatch: '#deaf8c' },
			{ value: 'ded98c', label: 'Light yellow', swatch: '#ded98c' },
			{ value: 'bade8c', label: 'Light lime', swatch: '#bade8c' },
			{ value: '91de8c', label: 'Light green', swatch: '#91de8c' },
			{ value: '8cdeaf', label: 'Light spring green', swatch: '#8cdeaf' },
			{ value: '8cded9', label: 'Light cyan', swatch: '#8cded9' },
			{ value: '8cbade', label: 'Light sky blue', swatch: '#8cbade' },
			{ value: '8c91de', label: 'Light blue', swatch: '#8c91de' },
			{ value: 'af8cde', label: 'Light violet', swatch: '#af8cde' },
			{ value: 'd98cde', label: 'Light magenta', swatch: '#d98cde' }
		]
	},
	{
		id: 'menuSpeed',
		label: 'Menu speed',
		help: 'Help text coming soon.',
		kind: 'choice',
		default: 'vanilla',
		options: [
			{ value: 'vanilla', label: 'Vanilla', desc: 'Original menu animations.' },
			{ value: 'fast', label: 'Fast', desc: 'Menus open and close quickly.' },
			{ value: 'instant', label: 'Instant', desc: 'No menu animations.' }
		]
	},
	{
		id: 'quickSwap',
		label: 'Quick swap',
		kind: 'toggle',
		default: false,
		help: 'Cycle through your items with the shoulder buttons without opening the menu.'
	},
	{
		id: 'customController',
		label: 'Custom controller layout',
		kind: 'toggle',
		default: false,
		help: 'Remap which button does what.',
		detailsWhen: true,
		details: [
			button('customController.sword', 'Sword', 'b', 'Help text coming soon.'),
			button('customController.item', 'Use item', 'y', 'Help text coming soon.'),
			button('customController.dash', 'Dash', 'a', 'Help text coming soon.'),
			button('customController.map', 'Open map', 'x', 'Help text coming soon.'),
			button('customController.prevItem', 'Previous item', 'l', 'Help text coming soon.'),
			button('customController.nextItem', 'Next item', 'r', 'Help text coming soon.')
		]
	},
	{
		id: 'musicMode',
		label: 'Music',
		help: 'Help text coming soon.',
		kind: 'choice',
		default: 'vanilla',
		options: [
			{ value: 'vanilla', label: 'Vanilla', desc: 'Original soundtrack.' },
			{ value: 'disabled', label: 'Disabled', desc: 'Sound effects only.' }
		]
	},
	{
		id: 'lowHealthBeep',
		label: 'Low-health beep',
		help: 'Help text coming soon.',
		kind: 'choice',
		default: 'vanilla',
		options: [
			{ value: 'vanilla', label: 'Vanilla' },
			{ value: 'slow', label: 'Slow', desc: 'Beeps half as often.' },
			{ value: 'off', label: 'Off', desc: 'No beeping.' }
		]
	},
	{
		id: 'reducedFlashing',
		label: 'Reduced flashing',
		kind: 'toggle',
		default: false,
		help: 'Tones down bright screen flashes (lightning, boss hits, and similar).'
	},
	{
		id: 'screenShake',
		label: 'Screen shake',
		help: 'Help text coming soon.',
		kind: 'choice',
		default: 'vanilla',
		options: [
			{ value: 'vanilla', label: 'Vanilla' },
			{ value: 'reduced', label: 'Reduced' }
		]
	}
];

export const customizationPresets: Preset[] = [{ id: 'default', name: 'Default', values: {} }];
