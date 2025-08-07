<script lang="ts">
    import type { IProfileViewUnknown } from "$lib/types/views/profile";
    import ProfileUnknownNip05 from "./profile-unknown-nip05.svelte";
    import ProfileUnknownNpub from "./profile-unknown-npub.svelte";
    import ProfileUnknownPublicKey from "./profile-unknown-public-key.svelte";

    let { basis }: { basis: IProfileViewUnknown } = $props();
</script>

{#if basis.unknown && `public_key` in basis.unknown}
    <ProfileUnknownPublicKey basis={basis.unknown} />
{:else if basis.unknown && `npub` in basis.unknown}
    <ProfileUnknownNpub basis={basis.unknown} />
{:else if basis.unknown && `nip05` in basis.unknown}
    <ProfileUnknownNip05 basis={basis.unknown} />
{:else}
    <p class={`font-sans font-[400] text-base text-ly0-gl`}>
        {`profile not found`}
    </p>
{/if}
