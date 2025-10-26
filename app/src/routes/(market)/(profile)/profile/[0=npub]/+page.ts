import { _env } from "$lib/utils/_env";
import { load_profile_indexed } from "$lib/utils/profile";
import type { EntryGenerator, PageLoad } from "./$types";

const { RADROOTS_MARKET_RELAY_INDEXES_URL: idx_url } = _env;

export const entries: EntryGenerator = async () => {
    const indexes: string[] = await fetch(`${idx_url}/events/0/npub/indexes.json`).then(r => r.json());
    return indexes.map(i => ({ 0: i }));
};

export const load: PageLoad = async ({ fetch, params }) => {
    const { 0: npub } = params;
    return load_profile_indexed(fetch, "npub", npub);
};

export const prerender = true;
