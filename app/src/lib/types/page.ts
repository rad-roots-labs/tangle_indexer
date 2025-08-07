import type { RadrootsMetadataEventData } from "@radroots/radroots-common-bindings";

export type PageLoadProfileData = {
    public_key: string;
    npub?: string;
    metadata_event: RadrootsMetadataEventData;
}