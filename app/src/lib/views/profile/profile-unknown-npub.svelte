<script lang="ts">
    import type { IProfileViewUnknownNpub } from "$lib/types/views/profile";
    import { lib_nostr_npub_decode } from "@radroots/utils-nostr";
    import { error } from "@sveltejs/kit";
    import { onMount } from "svelte";
    import ProfileUnknownPublicKey from "./profile-unknown-public-key.svelte";

    let {
        basis,
    }: {
        basis: IProfileViewUnknownNpub;
    } = $props();

    let public_key: string | undefined = $state(undefined);

    onMount(async () => {
        public_key = lib_nostr_npub_decode(basis.npub);
        if (!public_key) error(404, `invalid:public_key:${public_key}`);
    });
</script>

{#if public_key}
    <ProfileUnknownPublicKey basis={{ public_key }} />
{:else}
    <p class={`font-sans font-[400] text-base text-ly0-gl`}>
        {`not a valid npub ${basis.npub}`}
    </p>
{/if}
