<script lang="ts">
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { home_menu_visible } from "$lib/stores/lib";
    import { locale, ls } from "$lib/utils/i18n";
    import { root_symbol } from "@radroots/utils";
    import type { Snippet } from "svelte";

    let { children }: { children: Snippet } = $props();
</script>

<div
    class={`flex flex-row h-[45px] desktop:h-[4.25rem] w-full px-4 desktop:px-12 justify-between items-center bg-white/80 border-b-2 border-b-gray-200`}
>
    <div class={`flex flex-row gap-4 justify-start items-center`}>
        <button
            class={`flex flex-row justify-center items-center`}
            onclick={async () => {
                if (page.url.pathname === `/`) home_menu_visible.toggle();
                else await goto(`/`);
            }}
        >
            <p
                class={`font-sans font-[700] text-cloak_grey text-3xl tracking-tightest`}
            >
                {root_symbol}
            </p>
        </button>
        <button
            class={`flex flex-row justify-center items-center`}
            onclick={async () => {
                if (page.url.pathname === `/`)
                    locale.set($locale === `en` ? `es` : `en`);
                else await goto(`/`);
            }}
        >
            <p
                class={`font-br font-[700] text-cloak_grey text-lg desktop:text-2xl max-desktop:-translate-y-[1px]`}
            >
                {`${$ls(`web.nav.label`, { default: "rad roots" })}`}
            </p>
        </button>
    </div>
    {@render children()}
</div>
