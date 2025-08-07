<script lang="ts">
    import type { IProfileViewUnknownPublicKey } from "$lib/types/views/profile";

    let {
        basis,
    }: {
        basis: IProfileViewUnknownPublicKey;
    } = $props();
</script>

<div class={`flex flex-col w-full px-4 pt-8 gap-4 justify-start items-start`}>
    <p class={`font-sans font-[400] text-base text-ly0-gl`}>
        {`profile`}
    </p>
    <p class={`font-sans font-[400] text-base text-ly0-gl break-all`}>
        {`public_key `}{basis.public_key}
    </p>
</div>
