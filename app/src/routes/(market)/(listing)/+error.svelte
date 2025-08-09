<script lang="ts">
    import { page } from "$app/state";
</script>

{#if page.error?.message}
    {page.status}{` `}{page.error.message}
{:else}
    {"missing page error message"}
{/if}
