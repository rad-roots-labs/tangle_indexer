<script lang="ts">
    import type { PageProps } from "./$types";

    let { data }: PageProps = $props();
</script>

<div
    class={`flex flex-col w-full px-4 gap-12 justify-start items-start break-all`}
>
    <p class={`font-sans font-[400] text-base text-ly0-gl`}>
        {JSON.stringify(data.manifest)}
    </p>
    {#each data.events as event}
        <p class={`font-sans font-[400] text-base text-ly0-gl`}>
            {JSON.stringify(event)}
        </p>
    {/each}
</div>
