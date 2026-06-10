// this file was created with @radroots gen-localised-routes

import { get_locales, type Locales } from "@radroots/locales";

export const locale_routes: Record<string, string> = {
    "/login": "/login",
    "/login/confirm": "/login/confirm",
    "/signup": "/signup",
    "/signup/confirm": "/signup/confirm",
    "/acceso": "/login",
    "/acceso/confirmar": "/login/confirm",
    "/régistrarse": "/signup",
    "/régistrarse/confirmar": "/signup/confirm",
    "/about": "/about",
    "/blog": "/blog",
    "/contact": "/contact",
    "/faq": "/faq",
    "/acerca": "/about",
    "/contacto": "/contact",
    "/profile": "/profile",
    "/perfil": "/profile",
    // (simple) routes
    "/map": "/map",
    "/mapa": "/map",
};

export type LocalisedRoutesSimpleEntries = {
    "map": string;
};

export const locales_routes_simple_map: Record<Locales, LocalisedRoutesSimpleEntries> = {
    en: {
        map: "/map"
    },
    es: {
        map: "/mapa"
    },
};

export const localise_simple_route = (locale: string, key: keyof LocalisedRoutesSimpleEntries): string => {
    const loc = get_locales(locale);
    return locales_routes_simple_map[loc][key];
};

export const set_locales_routes_map = new Set(["map", "mapa"]);

export type LocalisedRoutesAuthEntries = {
    "login": string;
    "login_confirm": string;
    "signup": string;
    "signup_confirm": string;
};

export const locales_routes_auth_map: Record<Locales, LocalisedRoutesAuthEntries> = {
    en: {
        login: "/login",
        login_confirm: "/login/confirm",
        signup: "/signup",
        signup_confirm: "/signup/confirm",
    },
    es: {
        login: "/acceso",
        login_confirm: "/acceso/confirmar",
        signup: "/régistrarse",
        signup_confirm: "/régistrarse/confirmar",
    },
};
export const set_locales_routes_auth = new Set(["/login", "/login/confirm", "/signup", "/signup/confirm", "acceso", "acceso/confirmar", "régistrarse", "régistrarse/confirmar"]);

export const localise_auth_route = (locale: string, key: keyof LocalisedRoutesAuthEntries): string => {
    const loc = get_locales(locale);
    return locales_routes_auth_map[loc][key];
};

export type LocalisedRoutesInfoEntries = {
    "about": string;
    "blog": string;
    "contact": string;
    "faq": string;
};

export const locales_routes_info_map: Record<Locales, LocalisedRoutesInfoEntries> = {
    en: {
        about: "/about",
        blog: "/blog",
        contact: "/contact",
        faq: "/faq",
    },
    es: {
        about: "/acerca",
        blog: "/blog",
        contact: "/contacto",
        faq: "/faq",
    },
};
export const set_locales_routes_info = new Set(["/about", "/blog", "/contact", "/faq", "acerca", "blog", "contacto", "faq"]);

export const localise_info_route = (locale: string, key: keyof LocalisedRoutesInfoEntries): string => {
    const loc = get_locales(locale);
    return locales_routes_info_map[loc][key];
};

export type LocalisedRoutesProtectedEntries = {
    "profile": string;
};

export const locales_routes_protected_map: Record<Locales, LocalisedRoutesProtectedEntries> = {
    en: {
        profile: "/profile",
    },
    es: {
        profile: "/perfil",
    },
};
export const set_locales_routes_protected = new Set(["/profile", "perfil"]);

export const localise_protected_route = (locale: string, key: keyof LocalisedRoutesProtectedEntries): string => {
    const loc = get_locales(locale);
    return locales_routes_protected_map[loc][key];
};