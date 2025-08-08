import { PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL } from "$env/static/public";
import type { RadrootsListingEventData, RadrootsListingIndexCountryManifest } from "@radroots/radroots-common-bindings";
import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageLoad } from "./$types";

export const entries: EntryGenerator = async () => {
    const [
        events_0_country_indexes,
    ]: [
            string[]
        ] = await Promise.all([
            fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/30402/country/indexes.json`).then(r => r.json())
        ]);
    return events_0_country_indexes.map(i => ({ 0: i }))
};

type PageLoadData = {
    country: string;
    manifest: RadrootsListingIndexCountryManifest;
    events: RadrootsListingEventData[];
};

export const load: PageLoad<PageLoadData> = async ({ fetch, params }) => {
    const { 0: country } = params;

    const [
        res_country_manifest,
    ] = await Promise.all([
        fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/30402/country/${country}/manifest.json`)
    ]);

    if (!res_country_manifest.ok) error(404, { message: `country:${country}` });

    const manifest: RadrootsListingIndexCountryManifest = await res_country_manifest.json();

    let events: RadrootsListingEventData[] = [];
    if (manifest.shards.length > 0) {
        const shard = manifest.shards[0];
        const res_country_shard = await fetch(`${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/30402/country/${country}/${shard.file}?v=${shard.sha256}`);
        if (!res_country_shard.ok) error(500, { message: `load:${country}:${shard.file}` });
        events = await res_country_shard.json();
    }

    const data: PageLoadData = {
        country,
        manifest,
        events,
    }
    return data;
}

export const prerender = true;
