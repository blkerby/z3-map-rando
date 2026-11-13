import { loadMaster } from '$lib/master/load';
import type { PageLoad } from './$types';

export const load: PageLoad = async () => ({ master: await loadMaster() });
