<script lang="ts">
    import { locale, ls } from "$lib/utils/i18n";
    import { Glyph, Input } from "@radroots/apps-lib";

    let val_search = $state(``);

    const handle_search = async (): Promise<void> => {
        alert(`todo handle search`);
    };
</script>

<div class={`flex flex-row h-full w-full justify-start items-center`}>
    <Input
        bind:val={val_search}
        basis={{
            classes: `h-[22px] pl-2 bg-gray-200 text-[13px] font-rsfd placeholder:lowercase`,
            placeholder: `${$ls(`common.search`)} ${$locale}`,
            callback_keydown: async ({ is_submit }) => {
                if (is_submit) await handle_search();
            },
        }}
    />
    <button
        class={`flex flex-row h-[22px] flex-shrink-0 px-2 justify-center items-center bg-lime-500 hover:bg-lime-400`}
        onclick={async () => {
            locale.set($locale === `en` ? `es` : `en`);
        }}
    >
        <Glyph
            basis={{
                classes: `text-white`,
                size: `xs`,
                weight: `bold`,
                key: `magnifying-glass`,
            }}
        />
    </button>
</div>
