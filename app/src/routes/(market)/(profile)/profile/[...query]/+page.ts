import { REGEX_NOSTR_KEY } from "@radroots/nostr";
import { error } from "@sveltejs/kit";
import type { PageLoad } from "./$types";

export const load: PageLoad = async ({ params }) => {
    const { query } = params;

    let message = ``;
    if (query.startsWith(`npub`)) {
        message = `npub:${query}`;
    } else if (REGEX_NOSTR_KEY.test(query)) {
        message = `public_key:${query}`;
    } else {
        message = `nip05:${query}`;
    }
    error(404, { message });
}

export const ssr = false;
export const prerender = false;
