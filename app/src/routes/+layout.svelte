<script lang="ts">
    import { ndk, ndk_global } from "@radroots/apps-lib";
    import { idb, init_theme } from "@radroots/apps-lib-market";
    import { onMount, type Snippet } from "svelte";
    import "../app.css";

    let { children }: { children: Snippet } = $props();

    onMount(async () => {
        await init_theme();

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
    });
</script>

{@render children()}
