use crate::data::models::ParsedLine;

/// The chromatic scale with enharmonic equivalents.
const NOTES: &[&[&str]] = &[
    &["A"],
    &["A#", "Bb"],
    &["B", "Cb"],
    &["C", "B#"],
    &["C#", "Db"],
    &["D"],
    &["D#", "Eb"],
    &["E", "Fb"],
    &["F", "E#"],
    &["F#", "Gb"],
    &["G"],
    &["G#", "Ab"],
];

/// Transpose a single note name by the given number of half steps.
/// Returns the first enharmonic name for the resulting note.
pub fn transpose_note(note: &str, steps: i32) -> String {
    let note = note.trim();

    // Find the note in our table
    let index = NOTES.iter().position(|group| group.contains(&note));

    match index {
        Some(idx) => {
            let new_idx = ((idx as i32 + steps) % 12 + 12) % 12;
            NOTES[new_idx as usize][0].to_string()
        }
        None => {
            // Note not found, return unchanged
            note.to_string()
        }
    }
}

/// Transpose all chords in a set of parsed lines by the given number of half steps.
/// Returns new lines with transposed chords and updated content strings.
pub fn transpose_lines(lines: &[ParsedLine], steps: i32) -> Vec<ParsedLine> {
    if steps == 0 {
        return lines.to_vec();
    }

    lines
        .iter()
        .map(|line| {
            if line.chords.is_empty() {
                return line.clone();
            }

            let mut new_line = line.clone();
            let mut new_content = String::new();
            let mut last_end = 0;

            for chord in &mut new_line.chords {
                let old_root = &chord.root;
                let new_root = transpose_note(old_root, steps);
                let old_display = chord.display();

                // Build new chord display
                chord.root = new_root.clone();
                if let Some(ref mut bass) = chord.bass {
                    *bass = transpose_note(bass, steps);
                }
                let new_display = chord.display();

                // Reconstruct the content string
                // Add text up to this chord's column
                if chord.column > last_end {
                    // Pad with content from original
                    let original_segment = if chord.column <= line.content.len() {
                        &line.content[last_end..chord.column]
                    } else {
                        &line.content[last_end..]
                    };
                    new_content.push_str(original_segment);
                }

                new_content.push_str(&new_display);

                // Adjust spacing: if new chord name is shorter/longer, compensate
                let len_diff = new_display.len() as i32 - old_display.len() as i32;
                last_end = chord.column + old_display.len();

                // Update the column for this chord in the new line
                chord.column = new_content.len() - new_display.len();

                // Try to preserve spacing after the chord
                if len_diff < 0 {
                    // New chord is shorter, add extra spaces
                    for _ in 0..(-len_diff) {
                        new_content.push(' ');
                    }
                }
                // If new chord is longer, we may eat into the next space
                // (this is acceptable — exact alignment can shift slightly)
            }

            // Add remaining content after the last chord
            if last_end < line.content.len() {
                new_content.push_str(&line.content[last_end..]);
            }

            new_line.content = new_content;
            new_line
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpose_note_up() {
        assert_eq!(transpose_note("C", 2), "D");
        assert_eq!(transpose_note("G", 1), "G#");
        assert_eq!(transpose_note("B", 1), "C");
    }

    #[test]
    fn test_transpose_note_down() {
        assert_eq!(transpose_note("C", -1), "B");
        assert_eq!(transpose_note("A", -2), "G");
        assert_eq!(transpose_note("D", -3), "B");
    }

    #[test]
    fn test_transpose_note_wrap() {
        assert_eq!(transpose_note("G", 5), "C");
        assert_eq!(transpose_note("A", 12), "A"); // Full octave = same note
    }

    #[test]
    fn test_transpose_sharp() {
        assert_eq!(transpose_note("F#", 1), "G");
        assert_eq!(transpose_note("C#", -1), "C");
    }

    #[test]
    fn test_transpose_flat() {
        assert_eq!(transpose_note("Bb", 1), "B");
        assert_eq!(transpose_note("Eb", 2), "F");
    }

    #[test]
    fn test_unknown_note() {
        assert_eq!(transpose_note("X", 1), "X");
    }
}
