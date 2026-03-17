/// Integration tests for the API + parser pipeline.
/// These hit the real freetar proxy, so they require network access.

#[tokio::test]
async fn test_search() {
    // We need to test against the actual proxy to verify UA and parsing
    use once_cell::sync::Lazy;

    const SEARCH_PROXY: &str = "https://proxy.freetar.de/search.php";
    const USER_AGENT: &str = "python-requests/2.31.0";

    static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("Failed to build HTTP client")
    });

    let url = format!(
        "{}?page=1&search_type=title&value={}",
        SEARCH_PROXY, "wonderwall"
    );

    let resp = CLIENT.get(&url).send().await.expect("request failed");
    let status = resp.status();
    let text = resp.text().await.expect("failed to read body");

    println!("Search response status: {}", status);
    println!("Search response length: {} bytes", text.len());
    assert!(status.is_success(), "HTTP status: {}", status);

    // Check js-store div exists
    let document = scraper::Html::parse_document(&text);
    let selector = scraper::Selector::parse("div.js-store").unwrap();
    let store_div = document.select(&selector).next();
    assert!(store_div.is_some(), "js-store div not found");

    let data_content = store_div.unwrap().value().attr("data-content").unwrap();
    let data: serde_json::Value = serde_json::from_str(data_content).expect("invalid JSON");

    let results = data["store"]["page"]["data"]["results"]
        .as_array()
        .expect("no results array");
    println!("Found {} raw results", results.len());
    assert!(!results.is_empty(), "no search results");

    // Check first result has expected fields
    let first = &results[0];
    assert!(first["artist_name"].is_string());
    assert!(first["song_name"].is_string());
    assert!(first["tab_url"].is_string());
    println!(
        "First result: {} - {} ({})",
        first["artist_name"], first["song_name"], first["type"]
    );
}

#[tokio::test]
async fn test_fetch_tab() {
    use once_cell::sync::Lazy;

    const TAB_PROXY: &str = "https://tabs.proxy.freetar.de/tab/";
    const USER_AGENT: &str = "python-requests/2.31.0";

    static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
        reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("Failed to build HTTP client")
    });

    let url = format!("{}labi-siffre/bless-the-telephone-chords-1814395", TAB_PROXY);

    let resp = CLIENT.get(&url).send().await.expect("request failed");
    let status = resp.status();
    let text = resp.text().await.expect("failed to read body");

    println!("Tab response status: {}", status);
    println!("Tab response length: {} bytes", text.len());
    assert!(status.is_success(), "HTTP status: {}", status);

    let document = scraper::Html::parse_document(&text);
    let selector = scraper::Selector::parse("div.js-store").unwrap();
    let store_div = document.select(&selector).next();
    assert!(store_div.is_some(), "js-store div not found");

    let data_content = store_div.unwrap().value().attr("data-content").unwrap();
    let data: serde_json::Value = serde_json::from_str(data_content).expect("invalid JSON");

    let tab = &data["store"]["page"]["data"]["tab"];
    let tab_view = &data["store"]["page"]["data"]["tab_view"];

    assert_eq!(tab["artist_name"].as_str().unwrap(), "Labi Siffre");
    assert_eq!(tab["song_name"].as_str().unwrap(), "Bless The Telephone");

    let content = tab_view["wiki_tab"]["content"].as_str().unwrap();
    println!("Tab content length: {} chars", content.len());
    assert!(content.contains("[ch]"), "content should have [ch] markers");
    assert!(content.contains("[/ch]"), "content should have [/ch] markers");

    // Check applicature
    let applicature = &tab_view["applicature"];
    assert!(applicature.is_object(), "applicature should be an object");
    println!(
        "Chords in applicature: {:?}",
        applicature.as_object().unwrap().keys().collect::<Vec<_>>()
    );
}
