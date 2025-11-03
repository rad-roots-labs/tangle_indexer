import { _env } from "$lib/utils/_env";
import { type HttpFetch, fetch_json } from "@radroots/apps-lib";
import type { PageLoadProfileData } from "@radroots/apps-lib-market";
import type {
    RadrootsCommentEventMetadata,
    RadrootsListingEventMetadata,
    RadrootsProfileEventMetadata
} from "@radroots/events-bindings";
import type { RadrootsEventsIndexedManifest as radroots_events_indexed_manifest } from "@radroots/events-indexed-bindings";
import { lib_nostr_npub_encode } from "@radroots/utils-nostr";

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
): Promise<RadrootsListingEventMetadata[]> {
    const manifest = await fetch_json<radroots_events_indexed_manifest>(
        fetch_fn,
        `${idx_url}/events/30402/${kind}/${encodeURIComponent(key)}/manifest.json`
    );

    if (!manifest.shards.length) return [];

    const shard = manifest.shards[0];
    const shard_url = `${idx_url}/events/30402/${kind}/${encodeURIComponent(
        key
    )}/${shard.file}?v=${shard.sha256}`;
    return fetch_json<RadrootsListingEventMetadata[]>(fetch_fn, shard_url);
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
            try {
                const metas = await fetch_json<RadrootsCommentEventMetadata[]>(
                    fetch_fn,
                    url
                );
                return [id, metas];
            } catch {
                return [id, [] as RadrootsCommentEventMetadata[]];
            }
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
): Promise<PageLoadProfileDataWithComments> {
    const profile = await fetch_json<RadrootsProfileEventMetadata>(
        fetch_fn,
        `${idx_url}/events/0/${kind}/${encodeURIComponent(key)}/metadata.json`
    );

    const listings = await fetch_listings(fetch_fn, kind, key);
    const listingIds = listings.map((m) => m.id.toLowerCase());
    const listing_comments = await fetch_comments_for_roots(fetch_fn, listingIds);

    const public_key = profile.author;
    const npub = lib_nostr_npub_encode(public_key);

    return {
        public_key,
        npub,
        events: {
            profile,
            listings,
            listing_comments
        }
    };
}
