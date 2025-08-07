#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexerKey {
    Id,
    Author,
    Nip05,
    Npub,
    Geohash,
}

impl IndexerKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexerKey::Id => "id",
            IndexerKey::Author => "author",
            IndexerKey::Nip05 => "nip05",
            IndexerKey::Npub => "npub",
            IndexerKey::Geohash => "geohash",
        }
    }
}

pub const METADATA_INDEX_DIRECTORY: [IndexerKey; 4] = [
    IndexerKey::Id,
    IndexerKey::Author,
    IndexerKey::Nip05,
    IndexerKey::Npub,
];
