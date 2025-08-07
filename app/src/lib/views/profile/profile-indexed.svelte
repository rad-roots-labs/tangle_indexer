<script lang="ts">
    import type { IProfileViewIndexed } from "$lib/types/views/profile";
    import { head_title_suffix } from "$lib/utils/app/lib";
    import { NDKKind, type NDKUserProfile } from "@nostr-dev-kit/ndk";
    import { ndk } from "@radroots/apps-lib";
    import { on_ndk_event, type NdkEventPayload } from "@radroots/utils-nostr";

    let { basis }: { basis: IProfileViewIndexed } = $props();

    let ndk_profile: NDKUserProfile | null = $state(null);
    let ndk_events: NdkEventPayload[] = $state([]);

    $ndk.subscribe(
        {
            kinds: [NDKKind.Metadata],
            authors: [basis.index.public_key],
        },
        undefined,
        {
            onEvent: async (event) => {
                const ev = on_ndk_event(event);
                if (ev) ndk_events.push(ev);
            },
        },
    );

    const data_user = $derived(
        $ndk.getUser({ pubkey: basis.index.public_key }),
    );

    $effect(() => {
        data_user.fetchProfile().then((profile) => {
            if (profile) ndk_profile = profile;
        });
    });

    const head_title = $derived(
        `${
            basis.index.metadata_event.metadata.display_name ||
            basis.index.metadata_event.metadata.name
        } (@${basis.index.metadata_event.metadata.name}) ${head_title_suffix}`,
    );
</script>

<svelte:head>
    <title>
        {head_title}
    </title>
    <meta name="description" content={``} />
    <meta property="og:title" content={head_title} />
    <meta property="og:description" content={``} />
</svelte:head>

<div class={`flex flex-col w-full gap-12 justify-start items-start`}>
    <div class={`flex flex-col w-full gap-4 justify-start items-start`}>
        {#each Object.entries(basis.index.metadata_event) as [k, v]}
            <div class={`flex flex-col w-full gap-2 justify-start items-start`}>
                <p class={`font-sans font-[400] text-base text-ly0-gl`}>
                    {k}
                </p>
                <p
                    class={`font-sans font-[400] text-base text-ly0-gl break-all`}
                >
                    {JSON.stringify(v)}
                </p>
            </div>
        {/each}
    </div>
    <div class={`flex flex-col w-full gap-4 justify-start items-start`}>
        {#each ndk_events as ndk_event}
            <div
                class={`flex flex-col w-full gap-2 justify-start items-start break-all`}
            >
                {ndk_event.kind}
                {#if `metadata` in ndk_event}
                    {JSON.stringify(ndk_event.metadata)}
                {:else if `listing` in ndk_event}
                    {JSON.stringify(ndk_event.listing)}
                {/if}
            </div>
        {/each}
    </div>
</div>
