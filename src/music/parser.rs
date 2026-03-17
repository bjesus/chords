use regex::Regex;
use std::sync::LazyLock;

use crate::data::models::{ChordPosition, LineKind, ParsedLine};

static CHORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[ch\](?P<root>[A-Ha-h](#|b)?)(?P<quality>[^\[/]*)(?P<bass>/[A-Ha-h](#|b)?)?\[/ch\]")
        .unwrap()
});

static SECTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[(?:Intro|Verse|Chorus|Bridge|Outro|Solo|Interlude|Pre-Chorus|Post-Chorus|Hook|Instrumental|Break|Refrain|Coda|Ending|Riff|Tab|Capo)(?:\s*\d*)?\]").unwrap());

/// Parse raw tab content (with [ch]...[/ch] markers) into structured lines.
pub fn parse_tab_content(raw: &str) -> Vec<ParsedLine> {
    // Clean up the raw content
    let content = raw
        .replace("\r\n", "\n")
        .replace("[tab]", "")
        .replace("[/tab]", "");

    let mut lines = Vec::new();

    for raw_line in content.split('\n') {
        let line = parse_line(raw_line);
        lines.push(line);
    }

    lines
}

fn parse_line(raw_line: &str) -> ParsedLine {
    let trimmed = raw_line.trim();

    // Empty line
    if trimmed.is_empty() {
        return ParsedLine {
            kind: LineKind::Empty,
            content: String::new(),
            chords: Vec::new(),
        };
    }

    // Check for section header like [Verse 1], [Chorus], etc.
    if SECTION_RE.is_match(trimmed) {
        // Strip the chord markers if any exist inside headers (unlikely but safe)
        let clean = CHORD_RE.replace_all(raw_line, |caps: &regex::Captures| {
            format_chord_text(caps)
        });
        return ParsedLine {
            kind: LineKind::SectionHeader,
            content: clean.to_string(),
            chords: Vec::new(),
        };
    }

    // Check if this line contains chords
    if CHORD_RE.is_match(raw_line) {
        let mut chords = Vec::new();
        let mut plain_text = String::new();
        let mut last_end = 0;

        for caps in CHORD_RE.captures_iter(raw_line) {
            let m = caps.get(0).unwrap();

            // Add the text between the last chord and this one
            let between = &raw_line[last_end..m.start()];
            plain_text.push_str(between);

            let column = plain_text.len();

            let root = caps.name("root").map(|m| m.as_str()).unwrap_or("");
            let quality = caps.name("quality").map(|m| m.as_str()).unwrap_or("");
            let bass = caps.name("bass").map(|m| {
                let s = m.as_str();
                // Strip the leading '/'
                s.trim_start_matches('/').to_string()
            });

            let chord_text = format_chord_text(&caps);
            plain_text.push_str(&chord_text);

            chords.push(ChordPosition {
                column,
                root: root.to_string(),
                quality: quality.to_string(),
                bass,
            });

            last_end = m.end();
        }

        // Add remaining text after the last chord
        plain_text.push_str(&raw_line[last_end..]);

        // Determine if this is a chord-only line or a mixed line
        let non_chord_text = plain_text
            .chars()
            .filter(|c| !c.is_whitespace())
            .count();
        let chord_text_len: usize = chords.iter().map(|c| c.display().len()).sum();

        // If the non-whitespace content is mostly chords, it's a chord line
        let kind = if non_chord_text <= chord_text_len + 2 {
            LineKind::ChordLine
        } else {
            LineKind::LyricLine
        };

        ParsedLine {
            kind,
            content: plain_text,
            chords,
        }
    } else {
        // Pure lyric line (no chords)
        ParsedLine {
            kind: LineKind::LyricLine,
            content: raw_line.to_string(),
            chords: Vec::new(),
        }
    }
}

/// Format chord captures into plain text display.
fn format_chord_text(caps: &regex::Captures) -> String {
    let root = caps.name("root").map(|m| m.as_str()).unwrap_or("");
    let quality = caps.name("quality").map(|m| m.as_str()).unwrap_or("");
    let bass = caps.name("bass").map(|m| m.as_str()).unwrap_or("");
    format!("{}{}{}", root, quality, bass)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chord_line() {
        let line = "[ch]G[/ch]            [ch]D[/ch]                 [ch]Em[/ch]  [ch]Bm[/ch]";
        let parsed = parse_line(line);
        assert_eq!(parsed.kind, LineKind::ChordLine);
        assert_eq!(parsed.chords.len(), 4);
        assert_eq!(parsed.chords[0].root, "G");
        assert_eq!(parsed.chords[1].root, "D");
        assert_eq!(parsed.chords[2].root, "E");
        assert_eq!(parsed.chords[2].quality, "m");
        assert_eq!(parsed.chords[3].root, "B");
        assert_eq!(parsed.chords[3].quality, "m");
    }

    #[test]
    fn test_parse_lyric_line() {
        let line = "It's nice to hear your voice again";
        let parsed = parse_line(line);
        assert_eq!(parsed.kind, LineKind::LyricLine);
        assert!(parsed.chords.is_empty());
    }

    #[test]
    fn test_parse_section_header() {
        let line = "[Verse 1]";
        let parsed = parse_line(line);
        assert_eq!(parsed.kind, LineKind::SectionHeader);
    }

    #[test]
    fn test_parse_empty_line() {
        let parsed = parse_line("");
        assert_eq!(parsed.kind, LineKind::Empty);
    }

    #[test]
    fn test_chord_with_bass() {
        let line = "[ch]Am7/G[/ch]";
        let parsed = parse_line(line);
        assert_eq!(parsed.chords.len(), 1);
        assert_eq!(parsed.chords[0].root, "A");
        assert_eq!(parsed.chords[0].quality, "m7");
        assert_eq!(parsed.chords[0].bass.as_deref(), Some("G"));
    }

    #[test]
    fn test_parse_full_content() {
        let content = "[Verse 1]\n[ch]G[/ch]    [ch]D[/ch]\nHello world\n\n[Chorus]";
        let lines = parse_tab_content(content);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0].kind, LineKind::SectionHeader);
        assert_eq!(lines[1].kind, LineKind::ChordLine);
        assert_eq!(lines[2].kind, LineKind::LyricLine);
        assert_eq!(lines[3].kind, LineKind::Empty);
        assert_eq!(lines[4].kind, LineKind::SectionHeader);
    }
}
