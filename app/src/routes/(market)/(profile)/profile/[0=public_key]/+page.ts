import { _env } from "$lib/utils/_env";
import type { PageLoadProfileData } from "@radroots/apps-lib-market";
import type { RadrootsProfileEventMetadata } from "@radroots/events-bindings";
import { lib_nostr_npub_encode } from "@radroots/utils-nostr";
import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageLoad } from "./$types";

const { RADROOTS_MARKET_RELAY_INDEXES_URL: idx_url } = _env;

export const entries: EntryGenerator = async () => {
    const [
        events_0_author_indexes,
    ]: [
            string[]
        ] = await Promise.all([
            fetch(`${idx_url}/events/0/author/indexes.json`).then(r => r.json())
        ]);
    return events_0_author_indexes.map(i => ({ 0: i }))
};

type PageLoadData = PageLoadProfileData;

export const load: PageLoad<PageLoadData> = async ({ fetch, params }) => {
    const { 0: public_key } = params;

    const [
        res_author_metadata,
    ] = await Promise.all([
        fetch(`${idx_url}/events/0/author/${public_key}/metadata.json`),
    ]);

    if (!res_author_metadata.ok) error(404, { message: `public_key:${public_key}` });

    const profile_event: RadrootsProfileEventMetadata = await res_author_metadata.json();

    const npub = lib_nostr_npub_encode(public_key);

    const data: PageLoadData = {
        public_key,
        npub,
        events: {
            profile: profile_event
        }
    }

    return data;
}

export const prerender = true;
