import { _env } from "$lib/utils/_env";
import { type FetchJsonResult, type HttpFetch, fetch_json } from "@radroots/apps-lib";
import type { PageLoadProfileData } from "@radroots/apps-lib-market";
import type {
    RadrootsCommentEventMetadata,
    RadrootsListingEventMetadata,
    RadrootsProfileEventMetadata
} from "@radroots/events-bindings";
import type { RadrootsEventsIndexedManifest as radroots_events_indexed_manifest } from "@radroots/events-indexed-bindings";
import { nostr_npub_encode } from "@radroots/nostr";

type ProfileRoutesKind = "author" | "npub" | "nip05";

type CommentsByRoot = Record<string, RadrootsCommentEventMetadata[]>;

export type PageLoadProfileDataWithComments = PageLoadProfileData & {
    events: PageLoadProfileData["events"] & {
        listing_comments: CommentsByRoot;
    };
};

const { RADROOTS_MARKET_INDEXES_URL: idx_url } = _env;

async function fetch_listings(
    fetch_fn: HttpFetch,
    kind: ProfileRoutesKind,
    key: string
): Promise<FetchJsonResult<RadrootsListingEventMetadata[]>> {
    const manifest_res = await fetch_json<radroots_events_indexed_manifest>(
        fetch_fn,
        `${idx_url}/events/30402/${kind}/${encodeURIComponent(key)}/manifest.json`
    );

    if (!manifest_res.ok) return manifest_res;

    if (!manifest_res.data.shards.length) return { ok: true, data: [] };

    const shard = manifest_res.data.shards[0];
    const shard_url = `${idx_url}/events/30402/${kind}/${encodeURIComponent(
        key
    )}/${shard.file}?v=${shard.sha256}`;
    const events_res = await fetch_json<RadrootsListingEventMetadata[]>(fetch_fn, shard_url);
    if (!events_res.ok) return events_res;
    return { ok: true, data: events_res.data };
}

async function fetch_comments_for_roots(
    fetch_fn: HttpFetch,
    rootIds: readonly string[]
): Promise<CommentsByRoot> {
    const unique = Array.from(new Set(rootIds.map((id) => id.toLowerCase())));

    const entries: [string, RadrootsCommentEventMetadata[]][] = await Promise.all(
        unique.map(async (id): Promise<[string, RadrootsCommentEventMetadata[]]> => {
            const url = `${idx_url}/events/1111/root/${encodeURIComponent(
                id
            )}/metadata.json`;
            const metas_res = await fetch_json<RadrootsCommentEventMetadata[]>(
                fetch_fn,
                url
            );
            return [id, metas_res.ok ? metas_res.data : []];
        })
    );

    const out: CommentsByRoot = {};
    for (const [id, metas] of entries) {
        out[id] = metas;
    }

    return out;
}

export async function load_profile_indexed(
    fetch_fn: HttpFetch,
    kind: ProfileRoutesKind,
    key: string
): Promise<FetchJsonResult<PageLoadProfileDataWithComments>> {
    const profile_res = await fetch_json<RadrootsProfileEventMetadata>(
        fetch_fn,
        `${idx_url}/events/0/${kind}/${encodeURIComponent(key)}/metadata.json`
    );
    if (!profile_res.ok) return profile_res;

    const listings_res = await fetch_listings(fetch_fn, kind, key);
    if (!listings_res.ok) return listings_res;

    const listing_ids = listings_res.data.map((m) => m.id.toLowerCase());
    const listing_comments = await fetch_comments_for_roots(fetch_fn, listing_ids);

    const public_key = profile_res.data.author;
    const npub = nostr_npub_encode(public_key);

    return {
        ok: true,
        data: {
            public_key,
            npub,
            events: {
                profile: profile_res.data,
                listings: listings_res.data,
                listing_comments
            }
        }
    };
}
