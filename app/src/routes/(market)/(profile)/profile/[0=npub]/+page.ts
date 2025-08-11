import { _env } from "$lib/utils/_env";
import type { PageLoadProfileData } from "@radroots/apps-lib-market";
import type { RadrootsProfileEventMetadata } from "@radroots/radroots-common-bindings";
import { error } from "@sveltejs/kit";
import type { EntryGenerator, PageLoad } from "./$types";

const { RADROOTS_MARKET_RELAY_INDEXES_URL: indexes_url } = _env;

export const entries: EntryGenerator = async () => {
    const [
        events_0_author_indexes,
    ]: [
            string[]
        ] = await Promise.all([
            fetch(`${indexes_url}/events/0/npub/indexes.json`).then(r => r.json())
        ]);
    return events_0_author_indexes.map(i => ({ 0: i }))
};

type PageLoadData = PageLoadProfileData;

export const load: PageLoad<PageLoadData> = async ({ fetch, params }) => {
    const { 0: npub } = params;

    const [
        res_npub_metadata,
    ] = await Promise.all([
        fetch(`${indexes_url}/events/0/npub/${npub}/metadata.json`)
    ]);

    if (!res_npub_metadata.ok) error(404, { message: `npub:${npub}` });

    const profile_event: RadrootsProfileEventMetadata = await res_npub_metadata.json();

    const public_key = profile_event.author;

    const data: PageLoadData = {
        public_key,
        npub,
        events: {
            profile: profile_event
        }
    }
    return data;
}

export const prerender = true;
