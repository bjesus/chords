use once_cell::sync::Lazy;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashMap;

use super::models::*;

const SEARCH_PROXY: &str = "https://proxy.freetar.de/search.php";
const TAB_PROXY: &str = "https://tabs.proxy.freetar.de/tab/";
const USER_AGENT: &str = "python-requests/2.31.0";

/// Shared HTTP client with proper User-Agent.
static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to build HTTP client")
});

/// Fetch search results from the freetar proxy.
pub async fn search(query: &str, page: u32) -> Result<SearchResponse, ApiError> {
    let url = format!(
        "{}?page={}&search_type=title&value={}",
        SEARCH_PROXY,
        page,
        urlencoding::encode(query)
    );

    let resp = CLIENT.get(&url).send().await.map_err(ApiError::Network)?;
    let status = resp.status();
    let text = resp.text().await.map_err(ApiError::Network)?;

    if !status.is_success() {
        return Err(ApiError::Parse(format!(
            "Search returned HTTP {}: {}",
            status,
            &text[..text.len().min(200)]
        )));
    }

    let document = Html::parse_document(&text);
    let selector =
        Selector::parse("div.js-store").expect("invalid selector");
    let store_div = document
        .select(&selector)
        .next()
        .ok_or_else(|| {
            ApiError::Parse(format!(
                "Could not find js-store div (response length: {} bytes)",
                text.len()
            ))
        })?;

    let data_content = store_div
        .value()
        .attr("data-content")
        .ok_or_else(|| ApiError::Parse("No data-content attribute".into()))?;

    let data: Value = serde_json::from_str(data_content).map_err(ApiError::Json)?;

    let results_array = data["store"]["page"]["data"]["results"]
        .as_array()
        .ok_or_else(|| ApiError::Parse("No results array in JSON data".into()))?;

    let mut results = Vec::new();
    for item in results_array {
        let tab_type_str = item["type"].as_str().unwrap_or("");
        // Skip "Pro" and "Official" types, same as freetar
        if tab_type_str == "Pro" || tab_type_str == "Official" || tab_type_str.is_empty() {
            continue;
        }

        let tab_url_full = item["tab_url"].as_str().unwrap_or("");
        let tab_url = extract_path(tab_url_full);

        results.push(SearchResult {
            artist_name: item["artist_name"].as_str().unwrap_or("").to_string(),
            song_name: item["song_name"].as_str().unwrap_or("").to_string(),
            tab_url,
            tab_type: TabType::from_str(tab_type_str),
            version: item["version"].as_u64().unwrap_or(1) as u32,
            rating: round_rating(item["rating"].as_f64().unwrap_or(0.0)),
            votes: item["votes"].as_u64().unwrap_or(0) as u32,
        });
    }

    let pagination = &data["store"]["page"]["data"]["pagination"];
    let total_pages = pagination["total"].as_u64().unwrap_or(1) as u32;
    let current_page = pagination["current"].as_u64().unwrap_or(1) as u32;

    Ok(SearchResponse {
        results,
        total_pages,
        current_page,
    })
}

/// Fetch a single tab page from the freetar proxy.
pub async fn fetch_tab(url_path: &str) -> Result<TabData, ApiError> {
    // url_path is like "artist-slug/song-slug-chords-12345"
    // or "/tab/artist-slug/song-slug-chords-12345"
    let clean_path = url_path
        .trim_start_matches('/')
        .trim_start_matches("tab/");

    let url = format!("{}{}", TAB_PROXY, clean_path);

    let resp = CLIENT.get(&url).send().await.map_err(ApiError::Network)?;
    let status = resp.status();
    let text = resp.text().await.map_err(ApiError::Network)?;

    if !status.is_success() {
        return Err(ApiError::Parse(format!(
            "Tab fetch returned HTTP {}: {}",
            status,
            &text[..text.len().min(200)]
        )));
    }

    let document = Html::parse_document(&text);
    let selector =
        Selector::parse("div.js-store").expect("invalid selector");
    let store_div = document
        .select(&selector)
        .next()
        .ok_or_else(|| {
            ApiError::Parse(format!(
                "Could not find js-store div in tab page (response length: {} bytes)",
                text.len()
            ))
        })?;

    let data_content = store_div
        .value()
        .attr("data-content")
        .ok_or_else(|| ApiError::Parse("No data-content attribute".into()))?;

    let data: Value = serde_json::from_str(data_content).map_err(ApiError::Json)?;

    parse_tab_data(&data)
}

fn parse_tab_data(data: &Value) -> Result<TabData, ApiError> {
    let tab_data = &data["store"]["page"]["data"];
    let tab = &tab_data["tab"];
    let tab_view = &tab_data["tab_view"];

    let raw_content = tab_view["wiki_tab"]["content"]
        .as_str()
        .ok_or_else(|| ApiError::Parse("No tab content".into()))?
        .to_string();

    let artist_name = tab["artist_name"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();
    let song_name = tab["song_name"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();
    let version = tab["version"].as_u64().unwrap_or(1) as u32;
    let tab_type_str = tab["type"].as_str().unwrap_or("Chords");
    let tab_type = TabType::from_str(tab_type_str);
    let rating = round_rating(tab["rating"].as_f64().unwrap_or(0.0));
    let difficulty = tab_view["ug_difficulty"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // Capo
    let capo = if let Some(meta) = tab_view["meta"].as_object() {
        meta.get("capo")
            .and_then(|c| c.as_u64())
            .and_then(|c| if c > 0 { Some(c.to_string()) } else { None })
            .or_else(|| {
                meta.get("capo")
                    .and_then(|c| c.as_str())
                    .and_then(|s| {
                        if s == "0" || s.is_empty() {
                            None
                        } else {
                            Some(s.to_string())
                        }
                    })
            })
    } else {
        None
    };

    // Tuning
    let tuning = if let Some(meta) = tab_view["meta"].as_object() {
        meta.get("tuning").and_then(|t| {
            Some(TuningInfo {
                value: t["value"].as_str()?.to_string(),
                name: t["name"].as_str()?.to_string(),
            })
        })
    } else {
        None
    };

    let tab_url_full = tab["tab_url"].as_str().unwrap_or("");
    let tab_url = extract_path(tab_url_full);

    // Alternatives
    let mut alternatives = Vec::new();
    if let Some(versions) = tab_view["versions"].as_array() {
        for alt in versions {
            let alt_type = alt["type"].as_str().unwrap_or("");
            if alt_type == "Official" {
                continue;
            }
            let alt_url = extract_path(alt["tab_url"].as_str().unwrap_or(""));
            alternatives.push(SearchResult {
                artist_name: alt["artist_name"]
                    .as_str()
                    .unwrap_or(&artist_name)
                    .to_string(),
                song_name: alt["song_name"]
                    .as_str()
                    .unwrap_or(&song_name)
                    .to_string(),
                tab_url: alt_url,
                tab_type: TabType::from_str(alt_type),
                version: alt["version"].as_u64().unwrap_or(1) as u32,
                rating: round_rating(alt["rating"].as_f64().unwrap_or(0.0)),
                votes: alt["votes"].as_u64().unwrap_or(0) as u32,
            });
        }
    }

    // Applicature (chord fingering data)
    let applicature = parse_applicature(&tab_view["applicature"]);

    Ok(TabData {
        raw_content,
        artist_name,
        song_name,
        version,
        tab_type,
        rating,
        difficulty,
        capo,
        tuning,
        tab_url,
        alternatives,
        applicature,
    })
}

fn parse_applicature(value: &Value) -> Option<HashMap<String, Vec<ChordFingering>>> {
    let obj = value.as_object()?;
    let mut map = HashMap::new();

    for (chord_name, variants) in obj {
        let variants_arr = variants.as_array()?;
        let mut fingerings = Vec::new();

        for variant in variants_arr {
            let frets: Vec<i32> = variant["frets"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|v| v.as_i64().unwrap_or(-1) as i32)
                        .collect()
                })
                .unwrap_or_default();

            let fingers: Vec<i32> = variant["fingers"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|v| v.as_i64().unwrap_or(0) as i32)
                        .collect()
                })
                .unwrap_or_default();

            if !frets.is_empty() {
                fingerings.push(ChordFingering { frets, fingers });
            }
        }

        if !fingerings.is_empty() {
            map.insert(chord_name.clone(), fingerings);
        }
    }

    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

/// Extract just the path from a full URL.
fn extract_path(url_str: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url_str) {
        parsed.path().to_string()
    } else {
        // Already a relative path
        url_str.to_string()
    }
}

fn round_rating(r: f64) -> f64 {
    (r * 10.0).round() / 10.0
}

// =========================
// Deezer API — artist images
// =========================

const DEEZER_SEARCH: &str = "https://api.deezer.com/search/artist";

/// Fetch artist image bytes from Deezer. Returns the small (56x56) picture.
pub async fn fetch_artist_image(artist_name: &str) -> Result<Vec<u8>, ApiError> {
    let url = format!("{}?q={}&limit=1", DEEZER_SEARCH, urlencoding::encode(artist_name));

    let resp = CLIENT.get(&url).send().await.map_err(ApiError::Network)?;
    let text = resp.text().await.map_err(ApiError::Network)?;
    let data: Value = serde_json::from_str(&text).map_err(ApiError::Json)?;

    let picture_url = data["data"]
        .as_array()
        .and_then(|arr: &Vec<Value>| arr.first())
        .and_then(|artist: &Value| artist["picture_small"].as_str())
        .ok_or_else(|| ApiError::Parse("No artist image found on Deezer".into()))?
        .to_string();

    // Download the actual image
    let img_resp = CLIENT.get(&picture_url).send().await.map_err(ApiError::Network)?;
    let bytes = img_resp.bytes().await.map_err(ApiError::Network)?;
    Ok(bytes.to_vec())
}

#[derive(Debug)]
pub enum ApiError {
    Network(reqwest::Error),
    Parse(String),
    Json(serde_json::Error),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Network(e) => write!(f, "Network error: {}", e),
            ApiError::Parse(msg) => write!(f, "Parse error: {}", msg),
            ApiError::Json(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for ApiError {}
