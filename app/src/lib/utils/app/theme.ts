import { browser } from "$app/environment";
import { get_system_theme, theme_key, theme_mode, theme_set, theme_toggle } from "@radroots/apps-lib";
import { idb } from "./storage";

export const toggle_theme = async (): Promise<void> => {
    await theme_toggle(async (mode) => {
        await idb.save_global("theme_mode", mode);
    });
};

export const init_theme = async (): Promise<void> => {
    let mode = await idb.read_global("theme_mode");
    let key = await idb.read_global("theme_key");

    if (!mode) {
        mode = get_system_theme();
        await idb.save_global("theme_mode", mode);
    }

    if (!key) {
        key = `deault`;
        await idb.save_global("theme_key", key);
    }

    theme_mode.set(mode);
    theme_key.set(key);

    theme_set(key, mode);
};

theme_key.subscribe((key) => {
    theme_mode.subscribe((mode) => {
        if (browser) theme_set(key, mode);
    });
});