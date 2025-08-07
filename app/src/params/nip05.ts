import { set_routes_match_skip_all, set_routes_match_skip_index0 } from '$lib/utils/routes/lib';
import type { ParamMatcher } from '@sveltejs/kit';

export const match: ParamMatcher = (value: string): boolean => {
    return !!value && value.length !== 2
        && !set_routes_match_skip_all.has(value)
        && !set_routes_match_skip_index0.has(value)
};