<script lang="ts">
    import Nav from "$lib/components/nav.svelte";
    import type { LayoutProps } from "./$types";

    let { children }: LayoutProps = $props();
</script>

<div
    class={`relative flex flex-col min-h-[100vh] w-full justify-start items-center overflow-hidden bg-morning_snow cursor-default`}
>
    <Nav />
    {@render children()}
</div>
