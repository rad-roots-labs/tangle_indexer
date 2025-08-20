<script lang="ts">
    import { page } from "$app/state";
    import {
        Profile,
        type IProfileViewNetworkNip05,
        type IProfileViewNetworkNpub,
        type IProfileViewNetworkPublicKey,
    } from "@radroots/apps-lib-market";

    $effect(() => {
        console.log(`page.error`, page.error);
    });
</script>

{#if page.error?.message}
    {@const [profile_type, profile_payload] = page.error.message.split(`:`)}
    {@const unknown: IProfileViewNetworkPublicKey | IProfileViewNetworkNpub | IProfileViewNetworkNip05 | undefined =
        profile_type === `public_key`
            ? {
                  public_key: profile_payload,
              }
            : profile_type === `npub`
            ? {
                  npub: profile_payload,
              }
            : profile_type === `nip05`
            ? {
                  nip05: profile_payload,
              }
            : undefined}
    <Profile basis={{ unknown }} />
{:else}
    {"missing page error message"}
{/if}
