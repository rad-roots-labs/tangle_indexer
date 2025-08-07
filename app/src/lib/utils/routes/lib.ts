import { locale_routes } from "./localised.gen";

export const set_routes_match_skip_all = new Set([`favicon.ico`]);

export const set_routes_match_skip_index0 = new Set(Object.keys(locale_routes).filter(i => i.split(`/`).length === 2).map(i => i.split(`/`).pop()));
export const set_routes_match_skip_index1 = new Set(Object.keys(locale_routes).filter(i => i.split(`/`).length === 3).map(i => i.split(`/`).pop()));
