use thiserror::Error;

use radroots_core::{
    RadrootsCoreCurrency, RadrootsCoreDecimal, RadrootsCoreDiscount, RadrootsCoreMoney,
    RadrootsCoreQuantity, RadrootsCoreQuantityPrice, RadrootsCoreUnit,
};
use radroots_events::{
    kinds::{KIND_FARM, KIND_PLOT, KIND_RESOURCE_AREA},
    listing::{
        RadrootsListing, RadrootsListingAvailability, RadrootsListingBin,
        RadrootsListingDeliveryMethod, RadrootsListingEventIndex, RadrootsListingEventMetadata,
        RadrootsListingFarmRef, RadrootsListingImage, RadrootsListingImageSize,
        RadrootsListingLocation, RadrootsListingProduct, RadrootsListingStatus,
    },
    plot::RadrootsPlotRef,
    resource_area::RadrootsResourceAreaRef,
    RadrootsNostrEvent,
};

use crate::relay::event::RelayIndexerEvent;

#[derive(Debug, Error)]
pub enum RadrootsListingEventIndexError {
    #[error("Failed to parse listing from tags")]
    ParseError,
}

#[derive(Default)]
struct ListingBinDraft {
    quantity: Option<RadrootsCoreQuantity>,
    price_per_canonical_unit: Option<RadrootsCoreQuantityPrice>,
    display_amount: Option<RadrootsCoreDecimal>,
    display_unit: Option<RadrootsCoreUnit>,
    display_label: Option<String>,
    display_price: Option<RadrootsCoreMoney>,
    display_price_unit: Option<RadrootsCoreUnit>,
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
    let farm_pubkey = required(get_first("p"))?;
    let farm_pubkey = farm_pubkey.trim().to_string();
    if farm_pubkey.is_empty() {
        return Err(RadrootsListingEventIndexError::ParseError);
    }
    let parse_addr = |value: &str| -> Result<(u32, String, String), RadrootsListingEventIndexError> {
        let mut parts = value.splitn(3, ':');
        let kind = parts
            .next()
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or(RadrootsListingEventIndexError::ParseError)?;
        let pubkey = parts
            .next()
            .ok_or(RadrootsListingEventIndexError::ParseError)?
            .to_string();
        let d_tag = parts
            .next()
            .ok_or(RadrootsListingEventIndexError::ParseError)?
            .to_string();
        if pubkey.trim().is_empty() || d_tag.trim().is_empty() {
            return Err(RadrootsListingEventIndexError::ParseError);
        }
        Ok((kind, pubkey, d_tag))
    };

    let mut farm_addr_pubkey: Option<String> = None;
    let mut farm_d_tag: Option<String> = None;
    for tag in tags.iter().filter(|t| t.first().map(|k| k == "a").unwrap_or(false)) {
        let value = tag.get(1).ok_or(RadrootsListingEventIndexError::ParseError)?;
        let (kind, pubkey, d_tag) = parse_addr(value)?;
        if kind == KIND_FARM {
            farm_addr_pubkey = Some(pubkey);
            farm_d_tag = Some(d_tag);
            break;
        }
    }
    let farm_addr_pubkey = farm_addr_pubkey.ok_or(RadrootsListingEventIndexError::ParseError)?;
    let farm_d_tag = farm_d_tag.ok_or(RadrootsListingEventIndexError::ParseError)?;
    if farm_addr_pubkey != farm_pubkey || farm_d_tag.trim().is_empty() {
        return Err(RadrootsListingEventIndexError::ParseError);
    }
    let farm = RadrootsListingFarmRef {
        pubkey: farm_pubkey,
        d_tag: farm_d_tag,
    };

    let resource_area = if let Some(tag) = tags
        .iter()
        .find(|t| t.first().map(|k| k == "radroots:resource_area").unwrap_or(false))
    {
        let value = tag.get(1).ok_or(RadrootsListingEventIndexError::ParseError)?;
        let (kind, pubkey, d_tag) = parse_addr(value)?;
        if kind != KIND_RESOURCE_AREA {
            return Err(RadrootsListingEventIndexError::ParseError);
        }
        Some(RadrootsResourceAreaRef { pubkey, d_tag })
    } else {
        None
    };

    let plot = if let Some(tag) = tags
        .iter()
        .find(|t| t.first().map(|k| k == "radroots:plot").unwrap_or(false))
    {
        let value = tag.get(1).ok_or(RadrootsListingEventIndexError::ParseError)?;
        let (kind, pubkey, d_tag) = parse_addr(value)?;
        if kind != KIND_PLOT {
            return Err(RadrootsListingEventIndexError::ParseError);
        }
        Some(RadrootsPlotRef { pubkey, d_tag })
    } else {
        None
    };

    let location_tags: Vec<&Vec<String>> = tags
        .iter()
        .filter(|t| t.first().map(|k| k == "location").unwrap_or(false))
        .collect();
    let product_location = if location_tags.len() > 1 {
        location_tags.first().and_then(|t| t.get(1).cloned())
    } else {
        None
    };

    let product = RadrootsListingProduct {
        key: required(get_first("key"))?,
        title: required(get_first("title"))?,
        category: required(get_first("category"))?,
        summary: get_first("summary"),
        process: get_first("process"),
        lot: get_first("lot"),
        location: product_location,
        profile: get_first("profile"),
        year: get_first("year"),
    };

    let parse_decimal = |value: &str| value.parse::<RadrootsCoreDecimal>().ok();
    let parse_unit = |value: &str| value.parse::<RadrootsCoreUnit>().ok();
    let parse_currency = |value: &str| value.parse::<RadrootsCoreCurrency>().ok();

    let mut bin_order: Vec<String> = Vec::new();
    let mut bin_drafts: std::collections::BTreeMap<String, ListingBinDraft> =
        std::collections::BTreeMap::new();

    let mut upsert_bin = |bin_id: String, update: ListingBinDraft| {
        let entry = bin_drafts.entry(bin_id.clone()).or_default();
        if !bin_order.iter().any(|id| id == &bin_id) {
            bin_order.push(bin_id);
        }
        if update.quantity.is_some() {
            entry.quantity = update.quantity;
        }
        if update.price_per_canonical_unit.is_some() {
            entry.price_per_canonical_unit = update.price_per_canonical_unit;
        }
        if update.display_amount.is_some() {
            entry.display_amount = update.display_amount;
        }
        if update.display_unit.is_some() {
            entry.display_unit = update.display_unit;
        }
        if update.display_label.is_some() {
            entry.display_label = update.display_label;
        }
        if update.display_price.is_some() {
            entry.display_price = update.display_price;
        }
        if update.display_price_unit.is_some() {
            entry.display_price_unit = update.display_price_unit;
        }
    };

    for t in tags
        .iter()
        .filter(|t| t.first().map(|k| k == "radroots:bin").unwrap_or(false))
    {
        if t.len() < 4 {
            continue;
        }
        let bin_id = t.get(1).map(|v| v.trim().to_string()).unwrap_or_default();
        if bin_id.is_empty() {
            continue;
        }
        let amount = t.get(2).and_then(|v| parse_decimal(v));
        let unit = t.get(3).and_then(|v| parse_unit(v));
        let (Some(amount), Some(unit)) = (amount, unit) else {
            continue;
        };
        let mut draft = ListingBinDraft::default();
        draft.quantity = Some(RadrootsCoreQuantity {
            amount,
            unit,
            label: None,
        });
        let display_amount = t.get(4).and_then(|v| parse_decimal(v));
        let display_unit = t.get(5).and_then(|v| parse_unit(v));
        if let (Some(display_amount), Some(display_unit)) = (display_amount, display_unit) {
            draft.display_amount = Some(display_amount);
            draft.display_unit = Some(display_unit);
            let label = t
                .get(6)
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty());
            draft.display_label = label;
        }
        upsert_bin(bin_id, draft);
    }

    for t in tags
        .iter()
        .filter(|t| t.first().map(|k| k == "radroots:price").unwrap_or(false))
    {
        if t.len() < 6 {
            continue;
        }
        let bin_id = t.get(1).map(|v| v.trim().to_string()).unwrap_or_default();
        if bin_id.is_empty() {
            continue;
        }
        let money_amount = t.get(2).and_then(|v| parse_decimal(v));
        let money_currency = t.get(3).and_then(|v| parse_currency(v));
        let qty_amount = t.get(4).and_then(|v| parse_decimal(v));
        let qty_unit = t.get(5).and_then(|v| parse_unit(v));
        let (Some(money_amount), Some(money_currency), Some(qty_amount), Some(qty_unit)) =
            (money_amount, money_currency, qty_amount, qty_unit)
        else {
            continue;
        };
        let mut draft = ListingBinDraft::default();
        draft.price_per_canonical_unit = Some(RadrootsCoreQuantityPrice {
            amount: RadrootsCoreMoney {
                amount: money_amount,
                currency: money_currency,
            },
            quantity: RadrootsCoreQuantity {
                amount: qty_amount,
                unit: qty_unit,
                label: None,
            },
        });
        let display_amount = t.get(6).and_then(|v| parse_decimal(v));
        let display_unit = t.get(7).and_then(|v| parse_unit(v));
        if let (Some(display_amount), Some(display_unit)) = (display_amount, display_unit) {
            draft.display_price = Some(RadrootsCoreMoney {
                amount: display_amount,
                currency: money_currency,
            });
            draft.display_price_unit = Some(display_unit);
        }
        upsert_bin(bin_id, draft);
    }

    let bins: Vec<RadrootsListingBin> = bin_order
        .iter()
        .filter_map(|bin_id| bin_drafts.get(bin_id).map(|draft| (bin_id, draft)))
        .filter_map(|(bin_id, draft)| {
            let quantity = draft.quantity.clone()?;
            let price_per_canonical_unit = draft.price_per_canonical_unit.clone()?;
            Some(RadrootsListingBin {
                bin_id: bin_id.clone(),
                quantity,
                price_per_canonical_unit,
                display_amount: draft.display_amount,
                display_unit: draft.display_unit,
                display_label: draft.display_label.clone(),
                display_price: draft.display_price.clone(),
                display_price_unit: draft.display_price_unit,
            })
        })
        .collect();
    if bins.is_empty() {
        return Err(RadrootsListingEventIndexError::ParseError);
    }

    let primary_bin_id = required(get_first("radroots:primary_bin"))?
        .trim()
        .to_string();
    if primary_bin_id.is_empty() {
        return Err(RadrootsListingEventIndexError::ParseError);
    }
    if !bins.iter().any(|bin| bin.bin_id == primary_bin_id) {
        return Err(RadrootsListingEventIndexError::ParseError);
    }

    let mut primary: Option<String> = None;
    let mut city: Option<String> = None;
    let mut region: Option<String> = None;
    let mut country: Option<String> = None;
    if let Some(t) = location_tags.last() {
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

    let images = tags
        .iter()
        .filter(|t| t.first().map(|k| k == "image").unwrap_or(false))
        .map(|t| {
            let url = t.get(1).cloned().unwrap_or_default();
            let size = if t.len() >= 3 {
                let mut parts = t[2].split('x');
                let w = parts.next().and_then(|v| v.parse::<u32>().ok());
                let h = parts.next().and_then(|v| v.parse::<u32>().ok());
                if parts.next().is_none() {
                    match (w, h) {
                        (Some(w), Some(h)) => Some(RadrootsListingImageSize { w, h }),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };
            RadrootsListingImage { url, size }
        })
        .collect::<Vec<_>>();
    let images = if images.is_empty() { None } else { Some(images) };

    let inventory_available = get_first("inventory")
        .and_then(|value| parse_decimal(&value));

    let availability = if let Some(value) = get_first("status")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        let status = match value.as_str() {
            "active" => RadrootsListingStatus::Active,
            "sold" => RadrootsListingStatus::Sold,
            _ => RadrootsListingStatus::Other { value },
        };
        Some(RadrootsListingAvailability::Status { status })
    } else {
        let start = get_first("published_at").and_then(|v| v.parse::<u64>().ok());
        let end = get_first("expires_at").and_then(|v| v.parse::<u64>().ok());
        if start.is_some() || end.is_some() {
            Some(RadrootsListingAvailability::Window { start, end })
        } else {
            None
        }
    };

    let delivery_method = tags
        .iter()
        .find(|t| t.first().map(|k| k == "delivery").unwrap_or(false))
        .and_then(|t| t.get(1).map(|v| v.trim().to_string()))
        .and_then(|kind| {
            if kind.is_empty() {
                return None;
            }
            let method = match kind.as_str() {
                "pickup" => RadrootsListingDeliveryMethod::Pickup,
                "local_delivery" => RadrootsListingDeliveryMethod::LocalDelivery,
                "shipping" => RadrootsListingDeliveryMethod::Shipping,
                "other" => {
                    let detail = tags
                        .iter()
                        .find(|t| t.first().map(|k| k == "delivery").unwrap_or(false))
                        .and_then(|t| t.get(2))
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())?;
                    RadrootsListingDeliveryMethod::Other { method: detail }
                }
                _ => return None,
            };
            Some(method)
        });

    let mut discounts: Vec<RadrootsCoreDiscount> = Vec::new();
    for t in tags
        .iter()
        .filter(|t| t.first().map(|k| k == "radroots:discount").unwrap_or(false))
    {
        if let Some(payload) = t.get(1) {
            if let Ok(discount) = serde_json::from_str::<RadrootsCoreDiscount>(payload) {
                discounts.push(discount);
            }
        }
    }
    let discounts = if discounts.is_empty() { None } else { Some(discounts) };

    Ok(RadrootsListing {
        d_tag,
        farm,
        product,
        primary_bin_id,
        bins,
        resource_area,
        plot,
        discounts,
        inventory_available,
        availability,
        delivery_method,
        location,
        images,
    })
}

fn create_radroots_listing_event_metadata(
    id: String,
    author: String,
    published_at: u32,
    kind: u32,
    tags: &[Vec<String>],
) -> Result<RadrootsListingEventMetadata, RadrootsListingEventIndexError> {
    let listing = parse_listing_from_tags(tags)?;
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
        &self,
    ) -> Result<RadrootsListingEventIndex, RadrootsListingEventIndexError>;
}

impl ToRadrootsListingEventIndex for RelayIndexerEvent {
    fn to_radroots_listing_event(
        &self,
    ) -> Result<RadrootsListingEventIndex, RadrootsListingEventIndexError> {
        let kind_u32 = self.kind.as_u64() as u32;
        let id = self.id.clone();
        let author = self.author.clone();

        let metadata = create_radroots_listing_event_metadata(
            id.clone(),
            author.clone(),
            self.created_at,
            kind_u32,
            &self.tags,
        )?;

        Ok(RadrootsListingEventIndex {
            event: RadrootsNostrEvent {
                id,
                author,
                created_at: self.created_at,
                kind: kind_u32,
                tags: self.tags.clone(),
                content: self.content.clone(),
                sig: self.sig.clone(),
            },
            metadata,
        })
    }
}
