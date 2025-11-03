import { _env } from "$lib/utils/_env";
import { type HttpFetch, fetch_json } from "@radroots/apps-lib";
import type { PageLoadProfileData } from "@radroots/apps-lib-market";
import type { RadrootsListingEventMetadata, RadrootsProfileEventMetadata } from "@radroots/events-bindings";
import type { RadrootsEventsIndexedManifest as radroots_events_indexed_manifest } from "@radroots/events-indexed-bindings";
import { lib_nostr_npub_encode } from "@radroots/utils-nostr";

type ProfileRoutesKind = "author" | "npub" | "nip05";

const { RADROOTS_MARKET_INDEXES_URL: idx_url } = _env;

async function fetch_listings(fetch_fn: HttpFetch, kind: ProfileRoutesKind, key: string): Promise<RadrootsListingEventMetadata[]> {
    const manifest = await fetch_json<radroots_events_indexed_manifest>(
        fetch_fn,
        `${idx_url}/events/30402/${kind}/${key}/manifest.json`
    );
    if (!manifest.shards.length) return [];
    const shard = manifest.shards[0];
    const shard_url = `${idx_url}/events/30402/${kind}/${key}/${shard.file}?v=${shard.sha256}`;
    return fetch_json<RadrootsListingEventMetadata[]>(fetch_fn, shard_url);
}

export async function load_profile_indexed(fetch_fn: HttpFetch, kind: ProfileRoutesKind, key: string): Promise<PageLoadProfileData> {
    const profile = await fetch_json<RadrootsProfileEventMetadata>(
        fetch_fn,
        `${idx_url}/events/0/${kind}/${key}/metadata.json`
    );
    const listings = await fetch_listings(fetch_fn, kind, key);
    const public_key = profile.author;
    const npub = lib_nostr_npub_encode(public_key);
    return {
        public_key,
        npub,
        events: {
            profile,
            listings
        }
    };
}
