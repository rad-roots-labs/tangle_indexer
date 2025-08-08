use crate::domain::events::RequiredField;
use crate::relay::event::RelayIndexerEvent;
use crate::{opt_default, opt_required};
use anyhow::Result;
use radroots_common::models::events::{
    RadrootsListingEvent, RadrootsListingEventData, RadrootsNostrEvent,
};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RadrootsListingEventError {
    #[error("Missing or invalid tag structure")]
    TagParseError,

    #[error("Missing required field: {0}")]
    MissingField(String),
}

pub fn create_radroots_listing_event_data(
    id: String,
    author: String,
    tags: Vec<Vec<String>>,
) -> Result<RadrootsListingEventData, RadrootsListingEventError> {
    let mut tags_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut images = Vec::new();
    let mut location_lat: Option<String> = None;
    let mut location_lng: Option<String> = None;

    for tag in &tags {
        if let Some(key) = tag.get(0).map(String::as_str) {
            match key {
                "image" => {
                    if let Some(img) = tag.get(1) {
                        images.push(img.clone());
                    }
                }
                "l" => {
                    if let Some(value) = tag.get(1) {
                        if let Some(hint) = tag.get(2) {
                            match hint.as_str() {
                                "dd.lat" if location_lat.is_none() => {
                                    location_lat = Some(value.clone())
                                }
                                "dd.lon" if location_lng.is_none() => {
                                    location_lng = Some(value.clone())
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

    let published_at_str = opt_required!(get("published_at", 0))
        .map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let published_at = published_at_str
        .parse::<u32>()
        .map_err(|_| RadrootsListingEventError::MissingField("published_at".into()))?;

    let d_tag =
        opt_required!(get("d", 0)).map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let title =
        opt_required!(get("title", 0)).map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let summary =
        opt_required!(get("summary", 0)).map_err(|e| RadrootsListingEventError::MissingField(e))?;

    let location_address = opt_required!(get("location", 0))
        .map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let location_city = opt_required!(get("location", 1))
        .map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let location_region = opt_required!(get("location", 2))
        .map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let location_country = opt_required!(get("location", 3))
        .map_err(|e| RadrootsListingEventError::MissingField(e))?;

    let location_lat =
        opt_required!(location_lat).map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let location_lng =
        opt_required!(location_lng).map_err(|e| RadrootsListingEventError::MissingField(e))?;

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
        .ok_or_else(|| RadrootsListingEventError::MissingField("location_geohash".into()))?;

    let product_kind =
        opt_required!(get("key", 0)).map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let product_category = opt_required!(get("category", 0))
        .map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let product_process =
        opt_required!(get("process", 0)).map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let product_lot =
        opt_required!(get("lot", 0)).map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let product_profile = opt_default!(get("profile", 0));
    let product_year =
        opt_required!(get("year", 0)).map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let product_quantity_amt = opt_required!(get("quantity", 0))
        .map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let product_quantity_unit = opt_required!(get("quantity", 1))
        .map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let product_price_amt =
        opt_required!(get("price", 0)).map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let product_price_cur =
        opt_required!(get("price", 1)).map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let product_price_qty_amt =
        opt_required!(get("price", 2)).map_err(|e| RadrootsListingEventError::MissingField(e))?;
    let product_price_qty_unit =
        opt_required!(get("price", 3)).map_err(|e| RadrootsListingEventError::MissingField(e))?;

    Ok(RadrootsListingEventData {
        id,
        author,
        published_at,
        d_tag,
        title,
        summary,
        images,
        location_address,
        location_city,
        location_region,
        location_country,
        location_lat,
        location_lng,
        location_geohash,
        product_kind,
        product_category,
        product_process,
        product_lot,
        product_profile,
        product_year,
        product_quantity_amt,
        product_quantity_unit,
        product_price_amt,
        product_price_cur,
        product_price_qty_amt,
        product_price_qty_unit,
    })
}

pub trait ToRadrootsListingEvent {
    fn to_radroots_listing_event(self) -> Result<RadrootsListingEvent, RadrootsListingEventError>;
}

impl ToRadrootsListingEvent for RelayIndexerEvent {
    fn to_radroots_listing_event(self) -> Result<RadrootsListingEvent, RadrootsListingEventError> {
        let data = create_radroots_listing_event_data(
            self.id.clone(),
            self.author.clone(),
            self.tags.clone(),
        )?;

        Ok(RadrootsListingEvent {
            event: RadrootsNostrEvent {
                id: self.id,
                author: self.author,
                created_at: self.created_at,
                kind: self.kind.as_u64().try_into().unwrap(),
                content: self.content,
                tags: self.tags,
                sig: self.sig,
            },
            data,
        })
    }
}
