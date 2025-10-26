import { _env } from "$lib/utils/_env";
import { load_profile_indexed } from "$lib/utils/profile";
import type { EntryGenerator as entry_generator, PageLoad as page_load } from "./$types";

const { RADROOTS_MARKET_RELAY_INDEXES_URL: idx_url } = _env;

export const entries: entry_generator = async () => {
    const indexes: string[] = await fetch(`${idx_url}/events/0/nip05/indexes.json`).then(r => r.json());
    return indexes.map(i => ({ 0: i }));
};

export const load: page_load = async ({ fetch, params }) => {
    const { 0: nip05 } = params;
    return load_profile_indexed(fetch, "nip05", nip05);
};

export const prerender = true;
