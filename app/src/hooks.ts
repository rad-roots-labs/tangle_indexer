import { locale_routes } from "$lib/utils/routes/localised.gen";
import type { Reroute } from "@sveltejs/kit";

export const reroute: Reroute = ({ url }) => {
    const { pathname } = url;
    const segments = pathname.split("/");

    let modified = false;

    for (let i = 1; i < segments.length; i++) {
        const original = segments[i];
        const mapped = locale_routes[`/${original}`];

        if (mapped && mapped !== `/${original}`) {
            segments[i] = mapped.slice(1);
            modified = true;
        }
    }

    if (modified) {
        const new_path = segments.join("/");
        return new_path;
    }
};
