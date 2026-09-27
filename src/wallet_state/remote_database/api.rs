use super::RemoteWalletSnapshot;
use crate::network_activity::HttpRequestExt as _;
use crate::wallet_state::AddressBookEntry;
use crate::wallet_state::address_book::normalize_wallet_address_value;
use reqwest::{Client, Url};
use serde::Deserialize;
use std::collections::HashSet;
use std::time::Duration;

const PAGE_SIZE: usize = 200;
const MAX_RECORDS: usize = 100_000;
const MAX_PAGE_BYTES: usize = 2 * 1024 * 1024;

pub(crate) fn normalize_base_url(input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(String::new());
    }
    let url = Url::parse(input).map_err(|_| "Enter a valid HTTP or HTTPS base URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Enter a valid HTTP or HTTPS base URL".into());
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Use a base URL without credentials, query parameters, or a fragment".into());
    }
    if url.path().trim_end_matches('/').ends_with("/records")
        || url.path().trim_end_matches('/').ends_with("/_")
    {
        return Err("Enter the database base URL, without the records or admin path".into());
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WalletPage {
    page: usize,
    per_page: usize,
    total_pages: usize,
    total_items: usize,
    items: Vec<WalletRecord>,
}

#[derive(Deserialize)]
struct WalletRecord {
    id: String,
    address: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    expand: Expansion,
}

#[derive(Default, Deserialize)]
struct Expansion {
    entity: Option<Entity>,
}

#[derive(Deserialize)]
struct Entity {
    #[serde(default)]
    name: String,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

pub(crate) async fn fetch_wallets(base_url: String) -> Result<RemoteWalletSnapshot, String> {
    let base_url = normalize_base_url(&base_url)?;
    if base_url.is_empty() {
        return Err("No remote wallet database configured".into());
    }
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not initialize remote wallet connection".to_string())?;
    tokio::time::timeout(Duration::from_secs(60), fetch_pages(&client, &base_url))
        .await
        .map_err(|_| "Remote wallet sync timed out".to_string())?
}

async fn fetch_pages(client: &Client, base_url: &str) -> Result<RemoteWalletSnapshot, String> {
    let mut snapshot = RemoteWalletSnapshot::default();
    let mut record_ids = HashSet::new();
    let mut totals = None;
    for page_number in 1..=MAX_RECORDS {
        let mut response = client
            .get(format!("{base_url}/api/collections/wallets/records"))
            .header(reqwest::header::CACHE_CONTROL, "no-cache")
            .query(&[
                ("page", page_number.to_string()),
                ("perPage", PAGE_SIZE.to_string()),
                ("sort", "id".into()),
                ("expand", "entity".into()),
                (
                    "fields",
                    "id,address,label,tags,expand.entity.name,expand.entity.tags".into(),
                ),
            ])
            .send_observed()
            .await
            // reqwest errors can contain URLs; response bodies may contain private data.
            .map_err(|_| "Could not reach remote wallet database".to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "Remote wallet database returned HTTP {}",
                response.status().as_u16()
            ));
        }
        if response
            .content_length()
            .is_some_and(|size| size > MAX_PAGE_BYTES as u64)
        {
            return Err("Remote wallet response exceeds 2 MiB per page".into());
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Could not read remote wallet response".to_string())?
        {
            if body.len() + chunk.len() > MAX_PAGE_BYTES {
                return Err("Remote wallet response exceeds 2 MiB per page".into());
            }
            body.extend_from_slice(&chunk);
        }
        let page: WalletPage = serde_json::from_slice(&body)
            .map_err(|_| "Invalid PocketBase wallet response".to_string())?;
        if page.page != page_number
            || page.per_page == 0
            || page.per_page > PAGE_SIZE
            || page.total_items > MAX_RECORDS
            || page.total_pages != page.total_items.div_ceil(page.per_page)
            || totals
                .is_some_and(|value| value != (page.total_items, page.total_pages, page.per_page))
        {
            return Err(
                "Remote wallet pagination changed or is invalid; retrying next sync".into(),
            );
        }
        totals = Some((page.total_items, page.total_pages, page.per_page));
        let expected_count = page
            .total_items
            .saturating_sub((page_number - 1) * page.per_page)
            .min(page.per_page);
        if page.items.len() != expected_count {
            return Err("Incomplete remote wallet page; retrying next sync".into());
        }
        for record in page.items {
            if record.id.is_empty() || !record_ids.insert(record.id) {
                return Err("Duplicate or missing remote wallet record ID".into());
            }
            let Some(address) = normalize_wallet_address_value(&record.address) else {
                snapshot.skipped += 1;
                continue;
            };
            let entity = record.expand.entity;
            let label = if record.label.trim().is_empty() {
                entity
                    .as_ref()
                    .map(|entity| entity.name.trim())
                    .unwrap_or_default()
            } else {
                record.label.trim()
            }
            .to_string();
            let mut tags = Vec::new();
            for tag in record
                .tags
                .unwrap_or_default()
                .into_iter()
                .chain(entity.and_then(|entity| entity.tags).unwrap_or_default())
            {
                let tag = tag.trim().to_string();
                if !tag.is_empty() && !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
            if snapshot
                .entries
                .insert(
                    address,
                    AddressBookEntry {
                        label,
                        tags,
                        color: None,
                    },
                )
                .is_some()
            {
                return Err("Duplicate remote wallet address after normalization".into());
            }
        }
        if page_number >= page.total_pages {
            return Ok(snapshot);
        }
    }
    Err("Remote wallet database exceeds 100,000 records".into())
}

#[cfg(test)]
mod tests;
