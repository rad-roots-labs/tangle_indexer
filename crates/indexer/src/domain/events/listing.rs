use thiserror::Error;

use radroots_events::{
    listing::models::{
        RadrootsListing, RadrootsListingEventIndex, RadrootsListingEventMetadata,
        RadrootsListingImage, RadrootsListingImageSize, RadrootsListingLocation,
        RadrootsListingPrice, RadrootsListingProduct, RadrootsListingQuantity,
    },
    RadrootsNostrEvent,
};

use crate::relay::event::RelayIndexerEvent;

#[derive(Debug, Error)]
pub enum RadrootsListingEventIndexError {
    #[error("Failed to parse listing from tags")]
    ParseError,
}

fn parse_listing_from_tags(
    tags: &[Vec<String>],
) -> Result<RadrootsListing, RadrootsListingEventIndexError> {
    let get_first = |key: &str| -> Option<String> {
        tags.iter()
            .find(|t| {
                t.get(0)
                    .map(|s| s.eq_ignore_ascii_case(key))
                    .unwrap_or(false)
            })
            .and_then(|t| t.get(1).cloned())
    };

    let required = |v: Option<String>| v.ok_or(RadrootsListingEventIndexError::ParseError);

    let d_tag = required(get_first("d"))?;

    let product = RadrootsListingProduct {
        key: required(get_first("key"))?,
        title: required(get_first("title"))?,
        category: required(get_first("category"))?,
        summary: get_first("summary"),
        process: get_first("process"),
        lot: get_first("lot"),
        location: get_first("location"),
        profile: get_first("profile"),
        year: get_first("year"),
    };

    let mut quantities: Vec<RadrootsListingQuantity> = Vec::new();
    for t in tags
        .iter()
        .filter(|t| t.first().map(|k| k == "quantity").unwrap_or(false))
    {
        if t.len() >= 3 {
            let amount = match t[1].parse::<radroots_core::RadrootsCoreDecimal>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let unit = match t[2].parse::<radroots_core::RadrootsCoreUnit>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let label = t.get(3).cloned();
            quantities.push(RadrootsListingQuantity {
                value: radroots_core::RadrootsCoreQuantity {
                    amount,
                    unit,
                    label: label.clone(),
                },
                label,
                count: None,
            });
        }
    }

    let mut prices: Vec<RadrootsListingPrice> = Vec::new();
    for t in tags
        .iter()
        .filter(|t| t.first().map(|k| k == "price").unwrap_or(false))
    {
        if t.len() >= 5 {
            let money_amount = match t[1].parse::<radroots_core::RadrootsCoreDecimal>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let money_currency = match t[2].parse::<radroots_core::RadrootsCoreCurrency>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let qty_amount = match t[3].parse::<radroots_core::RadrootsCoreDecimal>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            let qty_unit = match t[4].parse::<radroots_core::RadrootsCoreUnit>() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let price = radroots_core::RadrootsCoreQuantityPrice {
                amount: radroots_core::RadrootsCoreMoney {
                    amount: money_amount,
                    currency: money_currency,
                },
                quantity: radroots_core::RadrootsCoreQuantity {
                    amount: qty_amount,
                    unit: qty_unit,
                    label: None,
                },
            };
            prices.push(price);
        }
    }

    let mut primary: Option<String> = None;
    let mut city: Option<String> = None;
    let mut region: Option<String> = None;
    let mut country: Option<String> = None;
    if let Some(t) = tags
        .iter()
        .find(|t| t.first().map(|k| k == "location").unwrap_or(false))
    {
        if t.len() >= 2 {
            primary = Some(t[1].clone());
        }
        if t.len() >= 3 {
            city = Some(t[2].clone());
        }
        if t.len() >= 4 {
            region = Some(t[3].clone());
        }
        if t.len() >= 5 {
            country = Some(t[4].clone());
        }
    }

    let geohash = tags
        .iter()
        .filter(|t| t.first().map(|k| k == "g").unwrap_or(false))
        .filter_map(|t| t.get(1).cloned())
        .max_by_key(|s| s.len());

    let mut lat: Option<f64> = None;
    let mut lng: Option<f64> = None;
    for t in tags.iter().filter(|t| {
        t.first()
            .map(|k| k.eq_ignore_ascii_case("l"))
            .unwrap_or(false)
    }) {
        if t.len() >= 3 {
            let val = t[1].parse::<f64>().ok();
            let label = t[2].as_str();
            match label {
                "dd.lat" => lat = val,
                "dd.lon" => lng = val,
                _ => {}
            }
        }
    }

    let location = if primary.is_some()
        || city.is_some()
        || region.is_some()
        || country.is_some()
        || lat.is_some()
        || lng.is_some()
        || geohash.is_some()
    {
        Some(RadrootsListingLocation {
            primary: primary.unwrap_or_default(),
            city,
            region,
            country,
            lat,
            lng,
            geohash,
        })
    } else {
        None
    };

    let images: Option<Vec<RadrootsListingImage>> = tags
        .iter()
        .filter(|t| t.first().map(|k| k == "img").unwrap_or(false))
        .map(|t| {
            let url = t.get(1).cloned().unwrap_or_default();
            let size = if t.len() >= 4 {
                let w = t[2].parse::<u32>().ok();
                let h = t[3].parse::<u32>().ok();
                match (w, h) {
                    (Some(w), Some(h)) => Some(RadrootsListingImageSize { w, h }),
                    _ => None,
                }
            } else {
                None
            };
            RadrootsListingImage { url, size }
        })
        .collect::<Vec<_>>()
        .into();

    Ok(RadrootsListing {
        d_tag,
        product,
        quantities,
        prices,
        discounts: None,
        location,
        images,
    })
}

fn create_radroots_listing_event_metadata(
    id: String,
    author: String,
    published_at: u32,
    kind: u32,
    _content: String,
    tags: Vec<Vec<String>>,
) -> Result<RadrootsListingEventMetadata, RadrootsListingEventIndexError> {
    let listing = parse_listing_from_tags(&tags)?;
    Ok(RadrootsListingEventMetadata {
        id,
        author,
        published_at,
        kind,
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
        let kind_u32 = self.kind.as_u64() as u32;

        let metadata = create_radroots_listing_event_metadata(
            self.id.clone(),
            self.author.clone(),
            self.created_at,
            kind_u32,
            self.content.clone(),
            self.tags.clone(),
        )?;

        Ok(RadrootsListingEventIndex {
            event: RadrootsNostrEvent {
                id: self.id,
                author: self.author,
                created_at: self.created_at,
                kind: kind_u32,
                tags: self.tags,
                content: self.content,
                sig: self.sig,
            },
            metadata,
        })
    }
}
