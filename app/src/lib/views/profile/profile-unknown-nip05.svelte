<script lang="ts">
    import type { IProfileViewUnknownNip05 } from "$lib/types/views/profile";
    import ProfileUnknownPublicKey from "./profile-unknown-public-key.svelte";

    let {
        basis,
    }: {
        basis: IProfileViewUnknownNip05;
    } = $props();

    let public_key: string | undefined = $state(undefined);
</script>

{#if public_key}
    <ProfileUnknownPublicKey basis={{ public_key }} />
{:else}
    <p class={`font-sans font-[400] text-base text-ly0-gl`}>
        {`could not find a public key for ${basis.nip05}`}
    </p>
{/if}
