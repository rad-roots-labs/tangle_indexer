<script lang="ts">
    import Profile from "$lib/views/profile/profile.svelte";
    import type { PageProps } from "./$types";

    let { data }: PageProps = $props();
</script>

<Profile basis={{ index: data }} />
