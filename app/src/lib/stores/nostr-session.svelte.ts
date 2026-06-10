import { browser } from "$app/environment";
import { NDKKind, NDKNip07Signer, NDKUser, type NDKUserProfile } from "@nostr-dev-kit/ndk";
import { get_store, ndk } from "@radroots/apps-lib";

export class NostrSession {
    user: NDKUser | null = $state(null);
    profile: NDKUserProfile | null = $state(null);
    follows: string[] = $state([]);
    settings: App.SettingsNip78 | null = $state(null);

    constructor(npub: string) {
        const ndk_store = get_store(ndk);
        this.user = ndk_store.getUser({ npub }) as unknown as NDKUser; //@todo
        if (this.user) {
            this.fetch_profile();
            this.fetch_follows();
            this.fetch_settings();
        }
    }

    async fetch_profile(): Promise<NDKUserProfile | null> {
        if (this.user) {
            const profile = await this.user.fetchProfile({});
            if (profile) this.profile = profile;
            return profile
        }
        return Promise.resolve(null);
    }

    async fetch_follows(): Promise<string[]> {
        if (this.user) {
            const follows_set = await this.user.followSet();
            const follows = Array.from(follows_set).map((public_key) => public_key);
            if (follows.length) this.follows = follows;
            return follows;
        }
        return Promise.resolve([]);
    }

    async fetch_settings(): Promise<App.SettingsNip78> {
        if (!this.user || !this.user.ndk) throw new Error(`[error] No nostr session user.`);
        if (!browser) return Promise.resolve({ dev_mode: false });
        const ndk = this.user.ndk;
        const events_app_data = await ndk.fetchEvents({
            kinds: [NDKKind.AppSpecificData],
            authors: [this.user.pubkey],
            "#d": [`radroots/settings/v1`],
        });
        const list_events_app_data = Array.from(events_app_data);

        let settings: App.SettingsNip78 = { dev_mode: false };
        if (list_events_app_data.length === 1) {
            const event_app_data = list_events_app_data[0];
            let signer: NDKNip07Signer;
            if (!ndk.signer) {
                signer = new NDKNip07Signer();
                ndk.signer = signer;
            }
            await event_app_data.decrypt(this.user);
            settings = JSON.parse(event_app_data.content);
        } else if (list_events_app_data.length > 1) {
            console.error(`[todo] Multiple app data settings events`, list_events_app_data)
        }
        return settings;
    }
}