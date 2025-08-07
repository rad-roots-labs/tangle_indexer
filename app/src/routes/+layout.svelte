<script lang="ts">
    import { PUBLIC_RADROOTS_MARKET_RELAY_URL } from "$env/static/public";
    import { init_theme } from "$lib/utils/app/theme";
    import { ndk } from "@radroots/apps-lib";
    import { onMount, type Snippet } from "svelte";
    import "../app.css";

    let { children }: { children: Snippet } = $props();

    onMount(async () => {
        await init_theme();

        $ndk.addExplicitRelay(PUBLIC_RADROOTS_MARKET_RELAY_URL);

        $ndk.autoConnectUserRelays = true;
        $ndk.autoFetchUserMutelist = true;

        await $ndk.connect();
    });
</script>

{@render children()}
