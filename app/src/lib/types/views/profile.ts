import type { PageLoadProfileData } from "../page";

export type IProfileView = IProfileViewIndexed | IProfileViewUnknown;

export type IProfileViewIndexed = {
    index: PageLoadProfileData;
};

export type IProfileViewUnknown = {
    unknown?: IProfileViewUnknownPublicKey | IProfileViewUnknownNpub | IProfileViewUnknownNip05;
};

export type IProfileViewUnknownPublicKey = {
    public_key: string;
};

export type IProfileViewUnknownNpub = {
    npub: string;
};

export type IProfileViewUnknownNip05 = {
    nip05: string;
};