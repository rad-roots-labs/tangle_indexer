import { writable } from "svelte/store";
import { NostrSession } from "./nostr-session.svelte";

export const home_menu_visible = (() => {
    const { subscribe, set, update } = writable<boolean>(false);
    return {
        subscribe,
        set,
        toggle: () => update(value => !value),
    };
})();

export const nostr_session = (() => {
    const { subscribe, set, update: _update } = writable<NostrSession | null>(null);

    return {
        subscribe,
        get: (): NostrSession | null => {
            let value: NostrSession | null;
            const unsubscribe = subscribe(v => (value = v));
            unsubscribe();
            return value!;
        },
        set: (npub: string | null): NostrSession | null => {
            const profile = npub ? new NostrSession(npub) : null;
            set(profile);
            return profile;
        },
    };
})();