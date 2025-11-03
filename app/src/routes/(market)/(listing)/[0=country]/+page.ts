import { _env } from "$lib/utils/_env";
import { load_listing_indexed } from "$lib/utils/listing";
import type { RadrootsListingEventMetadata } from "@radroots/events-bindings";
import type { RadrootsEventsIndexedManifest } from "@radroots/events-indexed-bindings";
import type { EntryGenerator, PageLoad } from "./$types";

const { RADROOTS_MARKET_INDEXES_URL: idx_url } = _env;

export const entries: EntryGenerator = async () => {
    if (!idx_url) return [];
    const indexes: string[] = await fetch(`${idx_url}/events/30402/country/indexes.json`).then((r) => r.json());
    return indexes.map((i) => ({ 0: i }));
};

type PageLoadData = {
    country: string;
    manifest: RadrootsEventsIndexedManifest;
    events: RadrootsListingEventMetadata[];
};

export const load: PageLoad<PageLoadData> = async ({ fetch, params }) => {
    const { 0: country } = params;
    const indexed = await load_listing_indexed(fetch, "country", country);

    return {
        country,
        manifest: indexed.manifest,
        events: indexed.events,
    };
};

export const prerender = idx_url ? true : false;
