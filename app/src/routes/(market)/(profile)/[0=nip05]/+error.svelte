<script lang="ts">
    import { page } from "$app/state";
</script>

<div class={`flex flex-col w-full justify-start items-start`}>
    <p class={`font-sans font-[400] text-base text-ly0-gl`}>
        {`error: ${page.error?.message}`}
    </p>
</div>
