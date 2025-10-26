<script lang="ts">
    import { Profile } from "@radroots/apps-lib-market";
    import type { PageProps } from "./$types";

    let { data }: PageProps = $props();

    $effect(() => {
        console.log(JSON.stringify(data, null, 4), `data`);
    });
</script>

<Profile basis={{ indexed: data }} />
