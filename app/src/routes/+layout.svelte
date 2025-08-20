<script lang="ts">
    import { ndk, ndk_global, ndk_user, nostr_login } from "@radroots/apps-lib";
    import { idb, init_theme } from "@radroots/apps-lib-market";
    import { lib_nostr_key_generate } from "@radroots/utils-nostr";
    import { onMount, type Snippet } from "svelte";
    import "../app.css";

    let { children }: { children: Snippet } = $props();

    let loaded = $state(false);

    onMount(async () => {
        await init_theme();

        loaded = true;

        await $ndk.connect();
        console.log(`[ndk] connected`);

        const global_relays = await idb.read_global("global_relays");
        if (!global_relays) {
            console.log(`[ndk_global] no global relays added`);
        } else {
            $ndk_global.explicitRelayUrls = global_relays;
        }

        await $ndk_global.connect();
        console.log(`[ndk_global] connected`);

        await nostr_login({
            nostr_key: lib_nostr_key_generate(),
        });

        console.log(`$ndk `, $ndk);
        console.log(`$ndk_user `, $ndk_user);
    });
</script>

{#if loaded}
    {@render children()}
{/if}
