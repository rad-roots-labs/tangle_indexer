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
            fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/0/nip05/indexes.json`).then(r => r.json())
        ]);
    return events_0_author_indexes.map(i => ({ 0: i }))
};

type PageLoadData = PageLoadProfileData;

export const load: PageLoad<PageLoadData> = async ({ fetch, params }) => {
    const { 0: nip05 } = params;

    const [
        res_nip05_metadata,
    ] = await Promise.all([
        fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/0/nip05/${nip05}/metadata.json`)
    ]);

    if (!res_nip05_metadata.ok) error(404, { message: `nip05:${nip05}`, });

    const metadata_event: RadrootsMetadataEventData = await res_nip05_metadata.json();

    const public_key = metadata_event.public_key;
    const npub = lib_nostr_npub_encode(public_key);

    const data: PageLoadData = {
        public_key,
        npub,
        metadata_event
    }
    return data;
}

export const prerender = true;
