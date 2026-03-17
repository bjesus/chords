use once_cell::sync::Lazy;
use std::collections::HashMap;

/// A chord voicing: 6 strings, low E (index 0) to high E (index 5).
/// None = muted (x), Some(0) = open, Some(n) = fretted at fret n.
pub type Voicing = [Option<i32>; 6];

/// The compiled-in chord database from Fretboard (GPL-3.0).
static CHORDS_TXT: &str = include_str!("../../data/chords.txt");

/// Parsed chord database: chord name → list of voicings (first = most common).
static CHORD_DB: Lazy<HashMap<String, Vec<Voicing>>> = Lazy::new(|| parse_database(CHORDS_TXT));

/// Look up the first (most common) voicing for a chord name.
/// Case-sensitive lookup first, then case-insensitive fallback.
pub fn get_voicing(chord_name: &str) -> Option<Voicing> {
    // Direct lookup
    if let Some(voicings) = CHORD_DB.get(chord_name) {
        return voicings.first().copied();
    }

    // Case-insensitive fallback
    let lower = chord_name.to_lowercase();
    for (key, voicings) in CHORD_DB.iter() {
        if key.to_lowercase() == lower {
            return voicings.first().copied();
        }
    }

    None
}

/// Look up all voicings for a chord name.
pub fn get_all_voicings(chord_name: &str) -> Vec<Voicing> {
    if let Some(voicings) = CHORD_DB.get(chord_name) {
        return voicings.clone();
    }

    let lower = chord_name.to_lowercase();
    for (key, voicings) in CHORD_DB.iter() {
        if key.to_lowercase() == lower {
            return voicings.clone();
        }
    }

    Vec::new()
}

/// Parse the chords.txt file format:
/// ```text
/// A
/// x 0 2 2 2 0
/// 5 7 7 6 5 5
///
/// Am
/// x 0 2 2 1 0
/// ...
/// ```
fn parse_database(text: &str) -> HashMap<String, Vec<Voicing>> {
    let mut db = HashMap::new();
    let mut current_name: Option<String> = None;
    let mut current_voicings: Vec<Voicing> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            // End of a chord block
            if let Some(name) = current_name.take() {
                if !current_voicings.is_empty() {
                    db.insert(name, std::mem::take(&mut current_voicings));
                }
            }
            continue;
        }

        if current_name.is_none() {
            // This line is a chord name
            current_name = Some(trimmed.to_string());
        } else {
            // This line is a voicing
            if let Some(voicing) = parse_voicing(trimmed) {
                current_voicings.push(voicing);
            }
        }
    }

    // Don't forget the last block
    if let Some(name) = current_name {
        if !current_voicings.is_empty() {
            db.insert(name, current_voicings);
        }
    }

    db
}

/// Parse a single voicing line like "x 0 2 2 2 0" into [Option<i32>; 6].
fn parse_voicing(line: &str) -> Option<Voicing> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() != 6 {
        return None;
    }

    let mut voicing = [None; 6];
    for (i, part) in parts.iter().enumerate() {
        voicing[i] = match *part {
            "x" | "X" => None,
            s => s.parse::<i32>().ok().map(Some)?,
        };
    }

    Some(voicing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_loads() {
        // Force lazy init
        let count = CHORD_DB.len();
        assert!(count > 1000, "Expected >1000 chords, got {}", count);
        println!("Loaded {} chords from database", count);
    }

    #[test]
    fn test_lookup_a() {
        let voicing = get_voicing("A").expect("A chord not found");
        // A = x 0 2 2 2 0 (first voicing)
        assert_eq!(voicing[0], None); // 6th string muted
        assert_eq!(voicing[1], Some(0)); // 5th string open
        assert_eq!(voicing[2], Some(2)); // 4th string fret 2
        assert_eq!(voicing[3], Some(2));
        assert_eq!(voicing[4], Some(2));
        assert_eq!(voicing[5], Some(0)); // 1st string open
    }

    #[test]
    fn test_lookup_am() {
        let voicing = get_voicing("Am").expect("Am chord not found");
        assert!(voicing[1] == Some(0)); // 5th string open
    }

    #[test]
    fn test_lookup_missing() {
        assert!(get_voicing("Xzz99").is_none());
    }

    #[test]
    fn test_multiple_voicings() {
        let voicings = get_all_voicings("A");
        assert!(voicings.len() > 1, "A should have multiple voicings");
        println!("A has {} voicings", voicings.len());
    }
}
