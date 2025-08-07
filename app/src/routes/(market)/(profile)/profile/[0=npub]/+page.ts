import { PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL } from "$env/static/public";
import type { PageLoadProfileData } from "$lib/types/page";
import type { RadrootsMetadataEventData } from "@radroots/radroots-common-bindings";
import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageLoad } from "./$types";

export const entries: EntryGenerator = async () => {
    const [
        events_0_author_indexes,
    ]: [
            string[]
        ] = await Promise.all([
            fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/0/npub/indexes.json`).then(r => r.json())
        ]);
    return events_0_author_indexes.map(i => ({ 0: i }))
};

type PageLoadData = PageLoadProfileData;

export const load: PageLoad<PageLoadData> = async ({ fetch, params }) => {
    const { 0: npub } = params;

    const [
        res_npub_metadata,
    ] = await Promise.all([
        fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/0/npub/${npub}/metadata.json`)
    ]);

    if (!res_npub_metadata.ok) error(404, { message: `npub:${npub}` });

    const metadata_event: RadrootsMetadataEventData = await res_npub_metadata.json();

    const public_key = metadata_event.public_key;

    const data: PageLoadData = {
        public_key,
        npub,
        metadata_event
    }
    return data;
}

export const prerender = true;
