import { PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL } from "$env/static/public";
import type { PageLoadProfileData } from "$lib/types/page";
import type { RadrootsMetadataEventData } from "@radroots/radroots-common-bindings";
import { lib_nostr_npub_encode } from "@radroots/utils-nostr";
import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageLoad } from "./$types";

export const entries: EntryGenerator = async () => {
    const [
        events_0_author_indexes,
    ]: [
            string[]
        ] = await Promise.all([
            fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/0/author/indexes.json`).then(r => r.json())
        ]);
    return events_0_author_indexes.map(i => ({ 0: i }))
};

type PageLoadData = PageLoadProfileData;

export const load: PageLoad<PageLoadData> = async ({ fetch, params }) => {
    const { 0: public_key } = params;

    const [
        res_author_metadata,
    ] = await Promise.all([
        fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/0/author/${public_key}/metadata.json`),
    ]);

    if (!res_author_metadata.ok) error(404, { message: `public_key:${public_key}` });

    const metadata_event: RadrootsMetadataEventData = await res_author_metadata.json();

    const npub = lib_nostr_npub_encode(public_key);

    const data: PageLoadData = {
        public_key,
        npub,
        events: {
            metadata: metadata_event
        }
    }

    return data;
}

export const prerender = true;
