import type { Group, Preset } from './types';

/**
 * Sample subset of the Super Metroid Map Rando settings.
 * Replace with the real ALTTP settings; the UI is generated from this.
 */
export const groups: Group[] = [
	{
		id: 'goal',
		title: 'Goal',
		icon: 'flag',
		settings: [
			{
				id: 'objectives',
				label: 'Objectives',
				kind: 'choice',
				display: 'cards',
				default: 'bosses',
				help: 'What you must complete before the final fight opens.',
				options: [
					{ value: 'none', label: 'None', desc: 'Go straight to the final boss.' },
					{ value: 'bosses', label: 'Bosses', desc: 'Defeat every major boss.' },
					{ value: 'minibosses', label: 'Minibosses', desc: 'Defeat every miniboss.' },
					{ value: 'chozos', label: 'Chozos', desc: 'Activate the Chozo statues.' },
					{ value: 'pirates', label: 'Pirates', desc: 'Clear the pirate rooms.' },
					{ value: 'metroids', label: 'Metroids', desc: 'Hunt down the Metroids.' },
					{ value: 'random', label: 'Random', desc: 'A random mix, chosen at generation.' }
				]
			},
			{
				id: 'saveAnimals',
				label: 'Save the animals',
				kind: 'choice',
				default: 'no',
				options: [
					{ value: 'no', label: 'No', desc: 'Skip the animal rescue.' },
					{ value: 'yes', label: 'Yes', desc: 'Required to finish.' },
					{ value: 'optional', label: 'Optional', desc: 'Available, not required.' },
					{ value: 'random', label: 'Random' }
				]
			}
		]
	},
	{
		id: 'world',
		title: 'World',
		icon: 'globe2',
		settings: [
			{
				id: 'mapLayout',
				label: 'Map layout',
				kind: 'choice',
				display: 'cards',
				default: 'standard',
				options: [
					{ value: 'vanilla', label: 'Vanilla', desc: 'The original map, unchanged.' },
					{ value: 'small', label: 'Small', desc: 'Fewer rooms, shorter runs.' },
					{ value: 'standard', label: 'Standard', desc: 'Fully shuffled rooms and paths.' },
					{ value: 'wild', label: 'Wild', desc: 'Few restrictions. Expect surprises.' }
				]
			},
			{
				id: 'doors',
				label: 'Doors',
				kind: 'choice',
				default: 'ammo',
				options: [
					{ value: 'blue', label: 'Blue', desc: 'No locked doors.' },
					{ value: 'ammo', label: 'Ammo', desc: 'Ammo-locked doors added.' },
					{ value: 'beam', label: 'Beam', desc: 'Ammo and beam-locked doors added.' }
				],
				details: [
					{
						id: 'doors.lockSize',
						label: 'Door lock size on map',
						kind: 'choice',
						default: 'large',
						options: [
							{ value: 'small', label: 'Small' },
							{ value: 'large', label: 'Large' }
						]
					}
				]
			},
			{
				id: 'startLocation',
				label: 'Start location',
				kind: 'choice',
				default: 'ship',
				options: [
					{ value: 'ship', label: 'Ship', desc: 'Start at the ship.' },
					{ value: 'random', label: 'Random', desc: 'Start somewhere random.' },
					{ value: 'escape', label: 'Escape', desc: 'Start at the escape point.' }
				]
			},
			{
				id: 'areaAssignment',
				label: 'Area assignment',
				kind: 'choice',
				default: 'standard',
				options: [
					{ value: 'standard', label: 'Standard' },
					{ value: 'size', label: 'Size' },
					{ value: 'depth', label: 'Depth' },
					{ value: 'random', label: 'Random' }
				]
			}
		]
	},
	{
		id: 'items',
		title: 'Items',
		icon: 'box-seam',
		settings: [
			{
				id: 'itemProgression',
				label: 'Item progression',
				kind: 'choice',
				display: 'cards',
				default: 'normal',
				options: [
					{ value: 'normal', label: 'Normal', desc: 'Comfortable item flow.' },
					{ value: 'tricky', label: 'Tricky', desc: 'Some items arrive later than you expect.' },
					{ value: 'technical', label: 'Technical', desc: 'Expect tech-heavy routes.' },
					{ value: 'challenge', label: 'Challenge', desc: 'Sparse, demanding progression.' },
					{ value: 'desolate', label: 'Desolate', desc: 'Very few items. Brutal.' }
				],
				details: [
					{
						id: 'itemProgression.rate',
						label: 'Progression rate',
						kind: 'choice',
						default: 'fast',
						options: [
							{ value: 'fast', label: 'Fast' },
							{ value: 'uniform', label: 'Uniform' },
							{ value: 'slow', label: 'Slow' }
						]
					},
					{
						id: 'itemProgression.placement',
						label: 'Item placement',
						kind: 'choice',
						default: 'neutral',
						options: [
							{ value: 'neutral', label: 'Neutral' },
							{ value: 'forced', label: 'Forced' },
							{ value: 'local', label: 'Local' }
						]
					},
					{
						id: 'itemProgression.priorityStrength',
						label: 'Item priority strength',
						kind: 'choice',
						default: 'moderate',
						options: [
							{ value: 'moderate', label: 'Moderate' },
							{ value: 'heavy', label: 'Heavy' }
						]
					},
					{ id: 'itemProgression.randomTankPriority', label: 'Random tank priority', kind: 'toggle', default: true },
					{ id: 'itemProgression.spazerBeforePlasma', label: 'Spazer before Plasma', kind: 'toggle', default: true },
					{
						id: 'itemProgression.ammoCollectFraction',
						label: 'Ammo collect fraction',
						kind: 'number',
						min: 0,
						max: 1,
						step: 0.05,
						default: 0.7
					}
				]
			}
		]
	},
	{
		id: 'difficulty',
		title: 'Difficulty',
		icon: 'speedometer2',
		settings: [
			{
				id: 'qol',
				label: 'Quality-of-life options',
				kind: 'choice',
				default: 'default',
				options: [
					{ value: 'off', label: 'Off' },
					{ value: 'low', label: 'Low' },
					{ value: 'default', label: 'Default' },
					{ value: 'high', label: 'High' },
					{ value: 'max', label: 'Max' }
				]
			}
		]
	},
	{
		id: 'misc',
		title: 'Misc',
		icon: 'sliders',
		settings: [
			{
				id: 'wallJump',
				label: 'Wall jump',
				kind: 'choice',
				default: 'vanilla',
				options: [
					{ value: 'vanilla', label: 'Vanilla' },
					{ value: 'collectible', label: 'Collectible', desc: 'Wall jump becomes an item.' }
				]
			},
			{
				id: 'speedBooster',
				label: 'Speed Booster',
				kind: 'choice',
				default: 'vanilla',
				options: [
					{ value: 'vanilla', label: 'Vanilla' },
					{ value: 'split', label: 'Split', desc: 'Split into separate abilities.' }
				]
			}
		]
	}
];

export const presets: Preset[] = [
	{ id: 'default', name: 'Default', values: {} },
	{
		id: 'hard',
		name: 'Hard mode',
		values: {
			itemProgression: 'challenge',
			mapLayout: 'wild',
			doors: 'beam',
			'itemProgression.rate': 'slow',
			qol: 'low'
		}
	},
	{
		id: 'relaxed',
		name: 'Relaxed',
		values: { mapLayout: 'small', objectives: 'none', qol: 'max' }
	}
];
