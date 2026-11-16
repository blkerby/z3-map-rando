import type { ChoiceOption } from '$lib/settings/types';

/**
 * Heart colors for the customization page's 10-column swatch picker: one column per hue, starting
 * from red. The first row starts with the vanilla red and is saturated, and the second row is
 * light. Each row's colors are adjusted by eye to roughly matching apparent brightness.
 */
export const HEART_COLORS: ChoiceOption[] = [
	{ value: 'c00000', label: 'Red (vanilla)', swatch: '#c00000' },
	{ value: 'c05300', label: 'Orange', swatch: '#c05300' },
	{ value: 'b2a700', label: 'Yellow', swatch: '#b2a700' },
	{ value: '00a30b', label: 'Green', swatch: '#00a30b' },
	{ value: '00a398', label: 'Cyan', swatch: '#00a398' },
	{ value: '0074cc', label: 'Sky blue', swatch: '#0074cc' },
	{ value: '3845ff', label: 'Blue', swatch: '#3845ff' },
	{ value: '7013eb', label: 'Violet', swatch: '#7013eb' },
	{ value: 'b300c0', label: 'Magenta', swatch: '#b300c0' },
	{ value: 'c0006a', label: 'Pink', swatch: '#c0006a' },
	{ value: 'de8c91', label: 'Light red', swatch: '#de8c91' },
	{ value: 'd7aa88', label: 'Light orange', swatch: '#d7aa88' },
	{ value: 'cac57f', label: 'Light yellow', swatch: '#cac57f' },
	{ value: '8bd586', label: 'Light green', swatch: '#8bd586' },
	{ value: '86d5d0', label: 'Light cyan', swatch: '#86d5d0' },
	{ value: '8cbade', label: 'Light sky blue', swatch: '#8cbade' },
	{ value: '8c91de', label: 'Light blue', swatch: '#8c91de' },
	{ value: 'af8cde', label: 'Light violet', swatch: '#af8cde' },
	{ value: 'd98cde', label: 'Light magenta', swatch: '#d98cde' },
	{ value: 'de8cba', label: 'Light pink', swatch: '#de8cba' }
];
