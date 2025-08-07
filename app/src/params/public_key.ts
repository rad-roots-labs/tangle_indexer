import { regex_nostr_key } from "@radroots/utils-nostr";
import type { ParamMatcher } from '@sveltejs/kit';

export const match: ParamMatcher = (value: string): boolean => {
    return regex_nostr_key.test(value);
};