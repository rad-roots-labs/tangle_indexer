import { REGEX_NOSTR_KEY } from "@radroots/nostr";
import type { ParamMatcher } from '@sveltejs/kit';

export const match: ParamMatcher = (value: string): boolean => {
    return REGEX_NOSTR_KEY.test(value);
};
