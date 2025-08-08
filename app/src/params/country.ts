import type { ParamMatcher } from '@sveltejs/kit';
export const match: ParamMatcher = (value: string): boolean => {
    return value.length == 2;
};