<script lang="ts">
    import DropdownNested from "$lib/components/dropdown-nested.svelte";
    import { nostr_session } from "$lib/stores/lib";
    import { locale, ls } from "$lib/utils/i18n";
    import { localise_auth_route } from "$lib/utils/routes/gen.tmp";
    import { type CallbackPromise } from "@radroots/utils";
    import NavSearch from "./nav-search.svelte";
    import NavSimple from "./nav-simple.svelte";

    const logout = async (): Promise<void> => {
        //nostr_session.set(null);
        // location.reload();
    };
</script>

<NavSimple>
    <div class={`flex flex-col justify-center items-center`}>
        <div
            class={`relative hidden desktop:flex flex-col h-[27px] w-[180px] justify-center items-center -translate-y-1`}
        >
            <NavSearch />
            {#if $nostr_session?.user}
                <div
                    class={`absolute -bottom-[1rem] flex flex-row w-full justify-start items-center`}
                >
                    <DropdownNested>
                        {#snippet primary()}
                            <p
                                class={`font-sans font-[400] text-xs text-layer-0-glyph line-clamp-1 overflow-hidden break-all hover:opacity-80`}
                            >
                                {#if $nostr_session.profile?.nip05}
                                    {`logged in as ${$nostr_session.profile.nip05}`}
                                {:else}
                                    {`logged in as ${$nostr_session.user?.npub}`}
                                {/if}
                            </p>
                        {/snippet}
                        {#snippet dropdown()}
                            <div
                                class={`flex flex-col w-full py-2 justify-center items-center`}
                            >
                                <div
                                    class={`flex flex-row w-full justify-center items-center`}
                                >
                                    <p
                                        class={`font-br font-[400] text-xs text-layer-0-glyph text-center`}
                                    >
                                        {`You are logged in via Nostr extension`}
                                    </p>
                                </div>
                            </div>
                            {#snippet anchor(href: string, label: string)}
                                <a
                                    {href}
                                    class={`flex flex-row w-full h-6 justify-center items-center hover:bg-gray-100`}
                                >
                                    <p
                                        class={`font-rsfd font-[400] text-xs text-layer-0-glyph capitalize`}
                                    >
                                        {label}
                                    </p>
                                </a>
                            {/snippet}
                            {@render anchor(
                                `/profile`, //@todo
                                `${$ls(`web.common.view_profile`)}`,
                            )}
                            {@render anchor(
                                `/`, //@todo
                                `${$ls(`web.common.settings`)}`,
                            )}
                        {/snippet}
                    </DropdownNested>
                </div>
            {:else}
                <div
                    class={`absolute -bottom-[1rem] flex flex-row w-full justify-start items-center`}
                >
                    <a
                        href={`${localise_auth_route($locale, `login`)}`}
                        class={`flex flex-row pl-1 justify-center items-center hover:opacity-40`}
                    >
                        <p
                            class={`font-br font-[400] text-[11px] text-black_panther/80 `}
                        >
                            {`${$ls(`web.nav.login.label`)}!!`}
                        </p>
                    </a>
                </div>
            {/if}
        </div>
        <div class={`desktop:hidden flex flex-row justify-start items-center`}>
            {#snippet button(label: string, callback: CallbackPromise)}
                <button
                    class={`flex flex-row w-full h-6 px-4 justify-center items-center bg-lime-500 hover:bg-lime-400 rounded-sm`}
                    onclick={async () => {
                        await callback();
                    }}
                >
                    <p
                        class={`font-br font-[700] text-xs text-white -translate-y-[1px]`}
                    >
                        {label}
                    </p>
                </button>
            {/snippet}
            {#if $nostr_session?.user}
                {@render button(`${$ls(`common.profile`)}`, async () => {
                    //
                })}
            {:else}
                {@render button(`${$ls(`common.log_in`)}`, async () => {
                    await logout();
                })}
            {/if}
        </div>
    </div>
</NavSimple>
