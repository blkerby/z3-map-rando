import type { Action } from 'svelte/action';
import type { Tooltip } from 'bootstrap';

/** Instant hover tooltip (Bootstrap), replacing the native `title` attribute and its built-in delay. No tooltip when `text` is empty. */
export const tooltip: Action<HTMLElement, string | undefined> = (node, text) => {
	let current = text;
	let tip: Tooltip | undefined;
	let destroyed = false;

	void import('bootstrap').then(({ Tooltip }) => {
		if (destroyed) return;
		tip = new Tooltip(node, { title: () => current ?? '', trigger: 'hover', placement: 'left', animation: false });
	});

	return {
		update(newText) {
			current = newText;
			if (!tip) return;
			if (newText) {
				tip.setContent({ '.tooltip-inner': newText });
			} else {
				tip.hide();
			}
		},
		destroy() {
			destroyed = true;
			tip?.dispose();
		}
	};
};
