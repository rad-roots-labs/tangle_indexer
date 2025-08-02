#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IndexerKey {
    Id,
    Author,
    Nip05,
    Geohash,
}

impl IndexerKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexerKey::Id => "id",
            IndexerKey::Author => "author",
            IndexerKey::Nip05 => "nip05",
            IndexerKey::Geohash => "geohash",
        }
    }
}

pub const METADATA_INDEX_DIRECTORY: [IndexerKey; 3] =
    [IndexerKey::Id, IndexerKey::Author, IndexerKey::Nip05];
