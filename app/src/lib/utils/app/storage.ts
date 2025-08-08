import { IdbLib, type ThemeMode } from "@radroots/apps-lib";

export type GlobalConfig = {
    theme_mode: ThemeMode;
    theme_key: string;
    locale: string;
    global_relays: string[];
    npub: string;
};

export type GlobalConfigKeys = keyof GlobalConfig;


export type PageSession = {
    draftId: string;
    lastScroll: number;
};

export type PageSessionKeys = keyof PageSession;

export const idb = new IdbLib<GlobalConfigKeys, GlobalConfig, PageSessionKeys, PageSession>();