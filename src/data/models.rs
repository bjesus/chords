use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single search result from Ultimate Guitar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub artist_name: String,
    pub song_name: String,
    pub tab_url: String,
    pub tab_type: TabType,
    pub version: u32,
    pub rating: f64,
    pub votes: u32,
}

/// The type of tab content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TabType {
    Chords,
    Tabs,
    Bass,
    Ukulele,
    Power,
    Drums,
    Other(String),
}

impl TabType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "chords" => TabType::Chords,
            "tabs" => TabType::Tabs,
            "bass tabs" => TabType::Bass,
            "ukulele chords" => TabType::Ukulele,
            "power" => TabType::Power,
            "drum tabs" => TabType::Drums,
            _ => TabType::Other(s.to_string()),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            TabType::Chords => "Chords",
            TabType::Tabs => "Tabs",
            TabType::Bass => "Bass",
            TabType::Ukulele => "Ukulele",
            TabType::Power => "Power",
            TabType::Drums => "Drums",
            TabType::Other(s) => s.as_str(),
        }
    }
}

/// Tuning information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningInfo {
    pub value: String,
    pub name: String,
}

/// Chord fingering data for a single chord variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChordFingering {
    pub frets: Vec<i32>,
    pub fingers: Vec<i32>,
}

/// Complete data for a single tab/chord page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabData {
    pub raw_content: String,
    pub artist_name: String,
    pub song_name: String,
    pub version: u32,
    pub tab_type: TabType,
    pub rating: f64,
    pub difficulty: String,
    pub capo: Option<String>,
    pub tuning: Option<TuningInfo>,
    pub tab_url: String,
    pub alternatives: Vec<SearchResult>,
    pub applicature: Option<HashMap<String, Vec<ChordFingering>>>,
}

/// Search response with pagination.
#[derive(Debug, Clone)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_pages: u32,
    pub current_page: u32,
}

/// A parsed chord with its position in a line.
#[derive(Debug, Clone)]
pub struct ChordPosition {
    pub column: usize,
    pub root: String,
    pub quality: String,
    pub bass: Option<String>,
}

impl ChordPosition {
    /// Full display name of the chord, e.g. "Am7/G".
    pub fn display(&self) -> String {
        let mut s = format!("{}{}", self.root, self.quality);
        if let Some(ref bass) = self.bass {
            s.push('/');
            s.push_str(bass);
        }
        s
    }
}

/// The kind of a parsed line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    ChordLine,
    LyricLine,
    SectionHeader,
    Empty,
}

/// A single parsed line from tab content.
#[derive(Debug, Clone)]
pub struct ParsedLine {
    pub kind: LineKind,
    pub content: String,
    pub chords: Vec<ChordPosition>,
}

/// A group of search results for the same song, with multiple versions/types.
#[derive(Debug, Clone)]
pub struct SongGroup {
    pub artist_name: String,
    pub song_name: String,
    /// All versions, sorted best-rated first.
    pub versions: Vec<SearchResult>,
}

impl SongGroup {
    /// The best version to show as the primary (first after sorting).
    pub fn best(&self) -> &SearchResult {
        &self.versions[0]
    }

    /// Merge new results into an existing list of groups.
    pub fn merge(groups: &mut Vec<SongGroup>, new_results: Vec<SearchResult>) {
        for result in new_results {
            let pos = groups.iter().position(|g| {
                g.artist_name == result.artist_name && g.song_name == result.song_name
            });
            if let Some(i) = pos {
                // Check for exact duplicate (same URL) before adding
                if !groups[i].versions.iter().any(|v| v.tab_url == result.tab_url) {
                    groups[i].versions.push(result);
                    // Re-sort: best rating first
                    groups[i].versions.sort_by(|a, b| {
                        b.rating.partial_cmp(&a.rating).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            } else {
                groups.push(SongGroup {
                    artist_name: result.artist_name.clone(),
                    song_name: result.song_name.clone(),
                    versions: vec![result],
                });
            }
        }
    }
}

/// Summary info for a saved tab (sidebar display).
#[derive(Debug, Clone)]
pub struct SavedTabSummary {
    pub tab_url: String,
    pub artist_name: String,
    pub song_name: String,
    pub tab_type: TabType,
    pub rating: f64,
    pub saved_at: i64,
}
