import { _env } from "$lib/utils/_env";
import { type FetchJsonResult, type HttpFetch, fetch_json } from "@radroots/apps-lib";
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
): Promise<FetchJsonResult<string[]>> {
    const url = `${idx_url}/events/30402/${kind}/indexes.json`;
    return fetch_json<string[]>(fetch_fn, url);
}

export async function load_listing_indexed(
    fetch_fn: HttpFetch,
    kind: ListingRoutesKind,
    key: string
): Promise<FetchJsonResult<ListingIndexedData>> {
    const manifest_url = `${idx_url}/events/30402/${kind}/${key}/manifest.json`;
    const manifest_res = await fetch_json<RadrootsEventsIndexedManifest>(fetch_fn, manifest_url);
    if (!manifest_res.ok) return manifest_res;

    let events: RadrootsListingEventMetadata[] = [];
    if (manifest_res.data.shards.length > 0) {
        const shard = manifest_res.data.shards[0];
        const shard_url = `${idx_url}/events/30402/${kind}/${key}/${shard.file}?v=${shard.sha256}`;
        const events_res = await fetch_json<RadrootsListingEventMetadata[]>(fetch_fn, shard_url);
        if (!events_res.ok) return events_res;
        events = events_res.data;
    }

    return { ok: true, data: { manifest: manifest_res.data, events } };
}
