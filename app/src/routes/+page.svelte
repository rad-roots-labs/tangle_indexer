<script lang="ts">
    import type { PageProps } from "./$types";

    let { data }: PageProps = $props();
</script>

<div class={`flex flex-col w-full gap-4 justify-start items-start`}>
    <div>radroots market</div>

    <div class={`flex flex-col w-full justify-start items-start`}>
        <p class={`font-sans font-[400] text-base text-ly0-gl`}>
            {`countries:`}
        </p>
        <div class={`flex flex-row gap-2 justify-start items-center`}>
            {#each data.countries as country}
                <a href={`/${country}`}>
                    <p class={`font-sans font-[400] text-base text-ly0-gl`}>
                        {country}
                    </p>
                </a>
            {/each}
        </div>
    </div>

    <div class={`flex flex-col w-full justify-start items-start`}>
        <p class={`font-sans font-[400] text-base text-ly0-gl`}>
            {`profiles:`}
        </p>
        <div class={`flex flex-col w-full gap-2 justify-start items-start`}>
            {#each data.profiles as profile}
                <a href={`/${profile}`}>
                    <p class={`font-sans font-[400] text-base text-ly0-gl`}>
                        {profile}
                    </p>
                </a>
            {/each}
        </div>
    </div>
</div>
