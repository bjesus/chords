use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

use super::models::*;

/// Thread-safe wrapper around the SQLite connection.
pub struct Cache {
    conn: Mutex<Connection>,
}

impl Cache {
    /// Open or create the cache database.
    pub fn open() -> Result<Self, CacheError> {
        let db_path = Self::db_path()?;

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CacheError::Io(format!("Failed to create data directory: {}", e))
            })?;
        }

        let conn = Connection::open(&db_path).map_err(CacheError::Sqlite)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(CacheError::Sqlite)?;

        let cache = Cache {
            conn: Mutex::new(conn),
        };
        cache.create_tables()?;
        Ok(cache)
    }

    pub fn db_path() -> Result<PathBuf, CacheError> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| CacheError::Io("Could not determine data directory".into()))?;
        Ok(data_dir.join("chords").join("chords.db"))
    }

    fn create_tables(&self) -> Result<(), CacheError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS saved_tabs (
                tab_url TEXT PRIMARY KEY,
                artist_name TEXT NOT NULL,
                song_name TEXT NOT NULL,
                tab_type TEXT NOT NULL,
                rating REAL NOT NULL DEFAULT 0.0,
                raw_content TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                saved_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS artist_images (
                artist_name TEXT PRIMARY KEY,
                image_data BLOB NOT NULL,
                fetched_at INTEGER NOT NULL
            );",
        )
        .map_err(CacheError::Sqlite)?;
        Ok(())
    }

    // =========================
    // Saved tabs
    // =========================

    pub fn save_tab(&self, tab: &TabData) -> Result<(), CacheError> {
        let conn = self.conn.lock().unwrap();
        let metadata_json =
            serde_json::to_string(tab).map_err(|e| CacheError::Io(e.to_string()))?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO saved_tabs
             (tab_url, artist_name, song_name, tab_type, rating, raw_content, metadata_json, saved_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                tab.tab_url,
                tab.artist_name,
                tab.song_name,
                tab.tab_type.display_name(),
                tab.rating,
                tab.raw_content,
                metadata_json,
                now,
            ],
        )
        .map_err(CacheError::Sqlite)?;
        Ok(())
    }

    pub fn remove_tab(&self, tab_url: &str) -> Result<(), CacheError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM saved_tabs WHERE tab_url = ?1", params![tab_url])
            .map_err(CacheError::Sqlite)?;
        Ok(())
    }

    pub fn get_saved_tab(&self, tab_url: &str) -> Result<Option<TabData>, CacheError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT metadata_json FROM saved_tabs WHERE tab_url = ?1")
            .map_err(CacheError::Sqlite)?;

        let result = stmt
            .query_row(params![tab_url], |row| {
                let json_str: String = row.get(0)?;
                Ok(json_str)
            })
            .optional()
            .map_err(CacheError::Sqlite)?;

        match result {
            Some(json_str) => {
                let tab: TabData =
                    serde_json::from_str(&json_str).map_err(|e| CacheError::Io(e.to_string()))?;
                Ok(Some(tab))
            }
            None => Ok(None),
        }
    }

    pub fn is_saved(&self, tab_url: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM saved_tabs WHERE tab_url = ?1",
            params![tab_url],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
            > 0
    }

    pub fn list_saved_tabs(&self) -> Result<Vec<SavedTabSummary>, CacheError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT tab_url, artist_name, song_name, tab_type, rating, saved_at
                 FROM saved_tabs ORDER BY saved_at DESC",
            )
            .map_err(CacheError::Sqlite)?;

        let rows = stmt
            .query_map([], |row| {
                Ok(SavedTabSummary {
                    tab_url: row.get(0)?,
                    artist_name: row.get(1)?,
                    song_name: row.get(2)?,
                    tab_type: TabType::from_str(&row.get::<_, String>(3)?),
                    rating: row.get(4)?,
                    saved_at: row.get(5)?,
                })
            })
            .map_err(CacheError::Sqlite)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(CacheError::Sqlite)?);
        }
        Ok(results)
    }

    // =========================
    // Artist images
    // =========================

    /// Get cached artist image bytes. Returns None if not cached.
    pub fn get_artist_image(&self, artist_name: &str) -> Option<Vec<u8>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT image_data FROM artist_images WHERE artist_name = ?1",
            params![artist_name],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .ok()
    }

    /// Save artist image bytes to the cache.
    pub fn save_artist_image(&self, artist_name: &str, image_data: &[u8]) {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let _ = conn.execute(
            "INSERT OR REPLACE INTO artist_images (artist_name, image_data, fetched_at)
             VALUES (?1, ?2, ?3)",
            params![artist_name, image_data, now],
        );
    }
}

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug)]
pub enum CacheError {
    Sqlite(rusqlite::Error),
    Io(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheError::Sqlite(e) => write!(f, "Database error: {}", e),
            CacheError::Io(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for CacheError {}
