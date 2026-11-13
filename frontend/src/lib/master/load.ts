import type { MasterData } from './master';
import local from './master_data.json';

/**
 * The one place the page gets its settings description from.
 *
 * Today: the file bundled with the site.
 * Later, when the backend exists, replace the body with a fetch and nothing else has to change:
 *
 *   const res = await fetch('/api/master');
 *   if (!res.ok) throw new Error(await res.text());
 *   return (await res.json()) as MasterData;
 */
export async function loadMaster(): Promise<MasterData> {
	return local as MasterData;
}
