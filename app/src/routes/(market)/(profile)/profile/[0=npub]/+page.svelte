<script lang="ts">
    import { Profile } from "@radroots/apps-lib-market";
    import type { PageProps } from "./$types";

    let { data }: PageProps = $props();
</script>

<Profile basis={{ indexed: data }} />
