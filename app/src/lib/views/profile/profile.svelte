<script lang="ts">
    import type { IProfileView } from "$lib/types/views/profile";
    import ProfileIndexed from "./profile-indexed.svelte";
    import ProfileUnknown from "./profile-unknown.svelte";

    let {
        basis,
    }: {
        basis: IProfileView;
    } = $props();
</script>

{#if `index` in basis}
    <ProfileIndexed {basis} />
{:else if `unknown` in basis && basis.unknown}
    <ProfileUnknown {basis} />
{/if}
