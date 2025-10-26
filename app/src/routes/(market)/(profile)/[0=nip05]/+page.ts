import { _env } from "$lib/utils/_env";
import type { PageLoadProfileData } from "@radroots/apps-lib-market";
import type { RadrootsListingEventMetadata, RadrootsProfileEventMetadata } from "@radroots/events-bindings";
import type { RadrootsEventsIndexedManifest } from "@radroots/events-indexed-bindings";
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
            fetch(`${idx_url}/events/0/nip05/indexes.json`).then(r => r.json())
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
        fetch(`${idx_url}/events/0/nip05/${nip05}/metadata.json`),
        fetch(`${idx_url}/events/30402/nip05/${nip05}/manifest.json`)
    ]);

    if (!res_nip05_metadata.ok) error(404, { message: `nip05:${nip05}`, });
    if (!res_nip05_listings_manifest.ok) error(404, { message: `nip05:listing:manifest:${nip05}`, });

    const profile_event: RadrootsProfileEventMetadata = await res_nip05_metadata.json();
    const listings_manifest: RadrootsEventsIndexedManifest = await res_nip05_listings_manifest.json();

    let listings_events: RadrootsListingEventMetadata[] = [];
    if (listings_manifest.shards.length > 0) {
        const shard = listings_manifest.shards[0];
        const res_country_shard = await fetch(`${idx_url}/events/30402/nip05/${nip05}/${shard.file}?v=${shard.sha256}`);
        if (!res_country_shard.ok) error(500, { message: `nip05:listing:shard:${nip05}:${shard.file}` });
        listings_events = await res_country_shard.json();
    }

    const public_key = profile_event.author;
    const npub = lib_nostr_npub_encode(public_key);

    const data: PageLoadProfileData = {
        public_key,
        npub,
        events: {
            profile: profile_event,
            listings: listings_events
        }
    }

    return data;
}

export const prerender = true;
