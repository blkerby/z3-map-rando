/**
 * One search box for the whole page. Both the Generate and Customization stores read
 * the same instance, so a single query filters both sections.
 */
export class SearchState {
	query = $state('');
}
