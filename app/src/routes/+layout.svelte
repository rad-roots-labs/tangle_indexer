<script lang="ts">
    import { idb, init_theme } from "@radroots/apps-lib-market";
    import { nostr_login_nip01 } from "@radroots/apps-nostr";
    import { nostr_context_create, nostr_context_default, nostr_key_generate, nostr_relays_clear, nostr_relays_open } from "@radroots/nostr";
    import { _env } from "$lib/utils/_env";
    import { onMount, type Snippet } from "svelte";
    import "../app.css";

    let { children }: { children: Snippet } = $props();

    let loaded = $state(false);
    const nostr_context = nostr_context_default();
    const nostr_context_global = nostr_context_create();

    onMount(async () => {
        await init_theme();

        loaded = true;

        const relay_urls = _env.RADROOTS_MARKET_RELAY_URL
            ? [_env.RADROOTS_MARKET_RELAY_URL]
            : [];
        nostr_relays_clear(nostr_context);
        if (relay_urls.length) nostr_relays_open(nostr_context, relay_urls);

        const global_relays = await idb.read_global("global_relays");
        nostr_relays_clear(nostr_context_global);
        if (!global_relays || !global_relays.length) {
            console.log(`[nostr] no global relays added`);
        } else {
            nostr_relays_open(nostr_context_global, global_relays);
        }
        nostr_login_nip01(nostr_key_generate());
    });
</script>

{#if loaded}
    {@render children()}
{/if}
