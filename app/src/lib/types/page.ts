import type { RadrootsListingEventData, RadrootsMetadataEventData } from "@radroots/radroots-common-bindings";

export type PageLoadProfileData = {
    public_key: string;
    npub?: string;
    events: PageLoadProfileDataEvents;
};

export type PageLoadProfileDataEvents =
    (
        {
            metadata: RadrootsMetadataEventData;
        } | {
            metadata: RadrootsMetadataEventData;
            listings: RadrootsListingEventData[];
        }
    );
