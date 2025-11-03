import { _env } from "$lib/utils/_env";
import { type HttpFetch, fetch_json } from "@radroots/apps-lib";
import type { RadrootsListingEventMetadata } from "@radroots/events-bindings";
import type { RadrootsEventsIndexedManifest } from "@radroots/events-indexed-bindings";

export type ListingRoutesKind = "country" | "author" | "npub" | "nip05";

const { RADROOTS_MARKET_INDEXES_URL: idx_url } = _env;

export type ListingIndexedData = {
    manifest: RadrootsEventsIndexedManifest;
    events: RadrootsListingEventMetadata[];
};

export async function fetch_listing_indexes(
    fetch_fn: HttpFetch,
    kind: ListingRoutesKind
): Promise<string[]> {
    const url = `${idx_url}/events/30402/${kind}/indexes.json`;
    return fetch_json<string[]>(fetch_fn, url);
}

export async function load_listing_indexed(
    fetch_fn: HttpFetch,
    kind: ListingRoutesKind,
    key: string
): Promise<ListingIndexedData> {
    const manifest_url = `${idx_url}/events/30402/${kind}/${key}/manifest.json`;
    const manifest = await fetch_json<RadrootsEventsIndexedManifest>(fetch_fn, manifest_url);

    let events: RadrootsListingEventMetadata[] = [];
    if (manifest.shards.length > 0) {
        const shard = manifest.shards[0];
        const shard_url = `${idx_url}/events/30402/${kind}/${key}/${shard.file}?v=${shard.sha256}`;
        events = await fetch_json<RadrootsListingEventMetadata[]>(fetch_fn, shard_url);
    }

    return { manifest, events };
}
