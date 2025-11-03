import { _env } from "$lib/utils/_env";
import { error } from "@sveltejs/kit";
import type { PageLoad } from "./$types";

const { RADROOTS_MARKET_INDEXES_URL: idx_url } = _env;

type PageLoadData = {
    profiles: string[];
    countries: string[];
};

export const load: PageLoad<PageLoadData> = async ({ fetch, params }) => {
    if (!idx_url) {
        return {
            profiles: [],
            countries: [],
        };
    }

    const [
        res_nip05_indexes,
        res_country_indexes,
    ] = await Promise.all([
        fetch(`${idx_url}/events/30402/nip05/indexes.json`),
        fetch(`${idx_url}/events/30402/country/indexes.json`),
    ]);

    if (!res_nip05_indexes.ok) error(404, { message: `nip05:indexes` });
    if (!res_country_indexes.ok) error(404, { message: `country:indexes` });

    const profiles: string[] = await res_nip05_indexes.json();
    const countries: string[] = await res_country_indexes.json();

    const data: PageLoadData = {
        profiles,
        countries,
    }
    return data;
}

export const prerender = idx_url ? true : false;
