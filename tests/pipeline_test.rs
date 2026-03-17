/// End-to-end pipeline test: search → fetch tab → parse → transpose

use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;

const SEARCH_PROXY: &str = "https://proxy.freetar.de/search.php";
const TAB_PROXY: &str = "https://tabs.proxy.freetar.de/tab/";
const USER_AGENT: &str = "python-requests/2.31.0";

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to build HTTP client")
});

static CHORD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[ch\](?P<root>[A-Ha-h](#|b)?)(?P<quality>[^\[/]*)(?P<bass>/[A-Ha-h](#|b)?)?\[/ch\]")
        .unwrap()
});

fn extract_js_store(html: &str) -> Value {
    let document = Html::parse_document(html);
    let selector = Selector::parse("div.js-store").unwrap();
    let store_div = document
        .select(&selector)
        .next()
        .expect("js-store div not found");
    let data_content = store_div
        .value()
        .attr("data-content")
        .expect("no data-content attribute");
    serde_json::from_str(data_content).expect("invalid JSON in data-content")
}

#[tokio::test]
async fn test_full_pipeline() {
    // Step 1: Search
    let url = format!(
        "{}?page=1&search_type=title&value=wonderwall",
        SEARCH_PROXY
    );
    let text = CLIENT.get(&url).send().await.unwrap().text().await.unwrap();
    let data = extract_js_store(&text);
    let results = data["store"]["page"]["data"]["results"]
        .as_array()
        .expect("no results array");

    // Find a "Chords" type result with a tab_url
    let chord_result = results
        .iter()
        .find(|r| {
            r["type"].as_str().unwrap_or("") == "Chords"
                && r["tab_url"].as_str().is_some()
        })
        .expect("No Chords result found");

    let tab_url_full = chord_result["tab_url"].as_str().unwrap();
    let path = url::Url::parse(tab_url_full)
        .map(|u| u.path().to_string())
        .unwrap_or_else(|_| tab_url_full.to_string());
    let clean_path = path.trim_start_matches('/').trim_start_matches("tab/");
    println!(
        "Step 1 - Found: {} - {} at {}",
        chord_result["artist_name"], chord_result["song_name"], clean_path
    );

    // Step 2: Fetch tab
    let tab_fetch_url = format!("{}{}", TAB_PROXY, clean_path);
    let text = CLIENT
        .get(&tab_fetch_url)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let data = extract_js_store(&text);
    let content = data["store"]["page"]["data"]["tab_view"]["wiki_tab"]["content"]
        .as_str()
        .expect("no tab content");
    println!("Step 2 - Tab content: {} chars", content.len());
    assert!(content.contains("[ch]"));

    // Step 3: Parse chords
    let clean = content
        .replace("\r\n", "\n")
        .replace("[tab]", "")
        .replace("[/tab]", "");

    let mut chord_count = 0;
    let mut unique_chords: std::collections::HashSet<String> = std::collections::HashSet::new();
    for caps in CHORD_RE.captures_iter(&clean) {
        let root = caps.name("root").map(|m| m.as_str()).unwrap_or("");
        let quality = caps.name("quality").map(|m| m.as_str()).unwrap_or("");
        unique_chords.insert(format!("{}{}", root, quality));
        chord_count += 1;
    }
    println!(
        "Step 3 - Found {} chord instances, {} unique: {:?}",
        chord_count,
        unique_chords.len(),
        unique_chords
    );
    assert!(chord_count > 0, "No chords found in tab content");

    // Step 4: Transpose
    let notes: &[&[&str]] = &[
        &["A"],     &["A#", "Bb"], &["B", "Cb"], &["C", "B#"],
        &["C#", "Db"], &["D"],    &["D#", "Eb"], &["E", "Fb"],
        &["F", "E#"],  &["F#", "Gb"], &["G"],    &["G#", "Ab"],
    ];

    for chord in &unique_chords {
        // Extract root note (1 or 2 chars)
        let root = if chord.len() >= 2
            && (chord.as_bytes()[1] == b'#' || chord.as_bytes()[1] == b'b')
        {
            &chord[..2]
        } else if !chord.is_empty() {
            &chord[..1]
        } else {
            continue;
        };
        let idx = notes.iter().position(|group| group.contains(&root));
        assert!(
            idx.is_some(),
            "Root '{}' from chord '{}' not found in note table",
            root, chord
        );
        let new_idx = ((idx.unwrap() as i32 + 2) % 12) as usize;
        println!(
            "Step 4 - Transpose {} +2 → {}{}",
            chord, notes[new_idx][0], &chord[root.len()..]
        );
    }

    // Step 5: Verify metadata
    let tab = &data["store"]["page"]["data"]["tab"];
    let artist = tab["artist_name"].as_str().unwrap();
    let song = tab["song_name"].as_str().unwrap();
    println!("Step 5 - Metadata: {} - {}", artist, song);
    assert!(!artist.is_empty());
    assert!(!song.is_empty());

    println!("\nFull pipeline test PASSED!");
}
