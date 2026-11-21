export interface ChoiceOption {
	value: string;
	label: string;
	/** Optional CSS colour; shown as a small dot beside the label (handy for colour pickers). */
	swatch?: string;
}

export interface LevelOption {
	value: number;
	label: string;
}
