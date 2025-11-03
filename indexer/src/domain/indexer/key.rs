#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexerKey {
    Id,
    Author,
    Nip05,
    Npub,
    Country,
    RootId,
}

impl IndexerKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexerKey::Id => "id",
            IndexerKey::Author => "author",
            IndexerKey::Nip05 => "nip05",
            IndexerKey::Npub => "npub",
            IndexerKey::Country => "country",
            IndexerKey::RootId => "root",
        }
    }
}

pub const PROFILE_INDEX_DIRECTORY: [IndexerKey; 4] = [
    IndexerKey::Id,
    IndexerKey::Author,
    IndexerKey::Nip05,
    IndexerKey::Npub,
];

pub const LISTING_INDEX_DIRECTORY: [IndexerKey; 5] = [
    IndexerKey::Id,
    IndexerKey::Country,
    IndexerKey::Author,
    IndexerKey::Npub,
    IndexerKey::Nip05,
];

pub const REACTION_INDEX_DIRECTORY: [IndexerKey; 5] = [
    IndexerKey::Id,
    IndexerKey::RootId,
    IndexerKey::Author,
    IndexerKey::Npub,
    IndexerKey::Nip05,
];
