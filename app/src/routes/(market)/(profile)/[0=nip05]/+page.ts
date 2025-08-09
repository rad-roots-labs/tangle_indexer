import { PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL } from "$env/static/public";
import type { PageLoadProfileData } from "$lib/types/page";
import type { RadrootsIndexManifest, RadrootsListingEventData, RadrootsMetadataEventData } from "@radroots/radroots-common-bindings";
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
        res_nip05_listings_manifest,
    ] = await Promise.all([
        fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/0/nip05/${nip05}/metadata.json`),
        fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/30402/nip05/${nip05}/manifest.json`)
    ]);

    if (!res_nip05_metadata.ok) error(404, { message: `nip05:${nip05}`, });
    if (!res_nip05_listings_manifest.ok) error(404, { message: `nip05:listing:manifest:${nip05}`, });

    const metadata_event: RadrootsMetadataEventData = await res_nip05_metadata.json();
    const listings_manifest: RadrootsIndexManifest = await res_nip05_listings_manifest.json();

    let listings_events: RadrootsListingEventData[] = [];
    if (listings_manifest.shards.length > 0) {
        const shard = listings_manifest.shards[0];
        const res_country_shard = await fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/30402/nip05/${nip05}/${shard.file}?v=${shard.sha256}`);
        if (!res_country_shard.ok) error(500, { message: `nip05:listing:shard:${nip05}:${shard.file}` });
        listings_events = await res_country_shard.json();
    }

    const public_key = metadata_event.public_key;
    const npub = lib_nostr_npub_encode(public_key);

    const data: PageLoadData = {
        public_key,
        npub,
        events: {
            metadata: metadata_event,
            listings: listings_events
        }
    }
    return data;
}

export const prerender = true;
