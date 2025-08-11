use crate::relay::event::RelayIndexerEvent;
use anyhow::Result;
use indexer_utils::nostr::NostrUtilsError;
use radroots_common::events::listing::models::{
    RadrootsListing, RadrootsListingDiscount, RadrootsListingEventIndex,
    RadrootsListingEventMetadata, RadrootsListingImage, RadrootsListingLocation,
    RadrootsListingPrice, RadrootsListingProduct, RadrootsListingQuantity,
};
use radroots_common::events::RadrootsNostrEvent;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RadrootsListingEventIndexError {
    #[error("Missing or invalid tag structure")]
    TagParseError,

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Nostr Error: {0}")]
    NostrUtilsError(#[from] NostrUtilsError),
}

pub fn create_radroots_listing_event_metadata(
    id: String,
    author: String,
    published_at: u32,
    tags: Vec<Vec<String>>,
) -> Result<RadrootsListingEventMetadata, RadrootsListingEventIndexError> {
    let mut tags_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut location_lat: Option<String> = None;
    let mut location_lng: Option<String> = None;

    for tag in &tags {
        if let Some(key) = tag.get(0).map(String::as_str) {
            match key {
                "l" => {
                    if let Some(value) = tag.get(1) {
                        if let Some(hint) = tag.get(2) {
                            match hint.as_str() {
                                "dd.lat" if location_lat.is_none() => {
                                    location_lat = Some(value.clone());
                                }
                                "dd.lon" if location_lng.is_none() => {
                                    location_lng = Some(value.clone());
                                }
                                _ => {}
                            }
                        } else if value.contains(',') {
                            let parts: Vec<&str> = value.split(',').map(str::trim).collect();
                            if parts.len() == 2 {
                                location_lat = Some(parts[0].to_string());
                                location_lng = Some(parts[1].to_string());
                            }
                        }
                    }
                }
                _ => {
                    tags_map
                        .entry(key.to_string())
                        .or_default()
                        .extend_from_slice(&tag[1..]);
                }
            }
        }
    }

    let get = |key: &str, idx: usize| -> Option<String> {
        tags_map.get(key).and_then(|v| v.get(idx)).cloned()
    };

    let d_tag =
        get("d", 0).ok_or_else(|| RadrootsListingEventIndexError::MissingField("d".into()))?;
    let title = get("title", 0)
        .ok_or_else(|| RadrootsListingEventIndexError::MissingField("title".into()))?;

    let location_address = get("location", 0)
        .ok_or_else(|| RadrootsListingEventIndexError::MissingField("location".into()))?;
    let location_city = get("location", 1)
        .ok_or_else(|| RadrootsListingEventIndexError::MissingField("location_city".into()))?;
    let location_region = get("location", 2)
        .ok_or_else(|| RadrootsListingEventIndexError::MissingField("location_region".into()))?;
    let location_country = get("location", 3)
        .ok_or_else(|| RadrootsListingEventIndexError::MissingField("location_country".into()))?;

    let location_geohash = tags
        .iter()
        .filter_map(|tag| {
            if tag.get(0).map(String::as_str) == Some("g") {
                tag.get(1).cloned()
            } else {
                None
            }
        })
        .max_by_key(|g| g.len())
        .ok_or_else(|| RadrootsListingEventIndexError::MissingField("location_geohash".into()))?;

    let quantities = tags
        .iter()
        .filter_map(|tag| {
            if tag.get(0).map(String::as_str) == Some("quantity") {
                Some(RadrootsListingQuantity {
                    amt: tag.get(1)?.clone(),
                    unit: tag.get(2)?.clone(),
                    label: tag.get(3).cloned(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<RadrootsListingQuantity>>();

    let prices = tags
        .iter()
        .filter_map(|tag| {
            if tag.get(0).map(String::as_str) == Some("price") {
                Some(RadrootsListingPrice {
                    amt: tag.get(1)?.clone(),
                    currency: tag.get(2)?.clone(),
                    qty_amt: tag.get(3)?.clone(),
                    qty_unit: tag.get(4)?.clone(),
                    qty_key: tag.get(5)?.clone(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<RadrootsListingPrice>>();

    let discounts = tags
        .iter()
        .filter_map(|tag| {
            if tag.get(0).map(String::as_str) == Some("discount") {
                Some(RadrootsListingDiscount::Quantity {
                    ref_quantity: "sample_ref".into(),
                    threshold: "100".into(),
                    value: "5".into(),
                    currency: "USD".into(),
                })
            } else {
                None
            }
        })
        .collect::<Vec<RadrootsListingDiscount>>();

    let location = Some(RadrootsListingLocation {
        primary: location_address,
        city: Some(location_city),
        region: Some(location_region),
        country: Some(location_country),
        lat: location_lat.map(|lat| lat.parse().unwrap_or_default()),
        lng: location_lng.map(|lng| lng.parse().unwrap_or_default()),
        geohash: Some(location_geohash),
    });

    let images = tags
        .iter()
        .filter_map(|tag| {
            if tag.get(0).map(String::as_str) == Some("image") {
                tag.get(1).map(|url| RadrootsListingImage {
                    url: url.clone(),
                    size: None,
                })
            } else {
                None
            }
        })
        .collect::<Vec<RadrootsListingImage>>();

    let product = RadrootsListingProduct {
        key: get("key", 0).unwrap_or_default(),
        title,
        category: get("category", 0).unwrap_or_default(),
        summary: get("summary", 0),
        process: get("process", 0),
        lot: get("lot", 0),
        location: get("location", 0),
        profile: get("profile", 0),
        year: get("year", 0),
    };

    let listing = RadrootsListing {
        d_tag,
        product,
        quantities,
        prices,
        discounts: Some(discounts),
        location,
        images: Some(images),
    };

    Ok(RadrootsListingEventMetadata {
        id,
        author,
        published_at,
        listing,
    })
}

pub trait ToRadrootsListingEventIndex {
    fn to_radroots_listing_event(
        self,
    ) -> Result<RadrootsListingEventIndex, RadrootsListingEventIndexError>;
}

impl ToRadrootsListingEventIndex for RelayIndexerEvent {
    fn to_radroots_listing_event(
        self,
    ) -> Result<RadrootsListingEventIndex, RadrootsListingEventIndexError> {
        let metadata = create_radroots_listing_event_metadata(
            self.id.clone(),
            self.author.clone(),
            self.created_at.clone(),
            self.tags.clone(),
        )?;

        Ok(RadrootsListingEventIndex {
            event: RadrootsNostrEvent {
                id: self.id,
                author: self.author,
                created_at: self.created_at,
                kind: self.kind.as_u64().try_into().unwrap(),
                content: self.content,
                tags: self.tags,
                sig: self.sig,
            },
            metadata,
        })
    }
}
