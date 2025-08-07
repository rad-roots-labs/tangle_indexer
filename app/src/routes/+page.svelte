<script lang="ts">
    import { PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL } from "$env/static/public";
    import { onMount } from "svelte";

    let nip05_profiles: any[] = $state([]);

    onMount(async () => {
        const res = await fetch(
            `${PUBLIC_RADROOTS_MARKET_RELAY_INDEXES_URL}/events/0/nip05/indexes.json`,
        );
        if (res.ok) nip05_profiles = await res.json();
    });
</script>

<div class={`flex flex-col w-full gap-4 justify-start items-start`}>
    <div>radroots market</div>
    <div class={`flex flex-col w-full gap-2 justify-start items-start`}>
        {#each nip05_profiles as nip05_profile}
            <a href={`/${nip05_profile}`}>
                <p class={`font-sans font-[400] text-base text-ly0-gl`}>
                    {nip05_profile}
                </p>
            </a>
        {/each}
    </div>
</div>
