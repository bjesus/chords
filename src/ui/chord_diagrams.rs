use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{self as gtk};

use crate::music::chord_db::{self, Voicing};
use crate::music::transpose::transpose_note;

/// Bottom panel showing chord diagrams looked up from a local database.
#[derive(Clone)]
pub struct ChordDiagramPanel {
    pub revealer: gtk::Revealer,
    flow_box: gtk::FlowBox,
    /// Original chord names from the tab (before transposition).
    chord_names: Rc<RefCell<Vec<String>>>,
    transpose_steps: Rc<Cell<i32>>,
}

impl ChordDiagramPanel {
    pub fn new() -> Self {
        let flow_box = gtk::FlowBox::new();
        flow_box.set_selection_mode(gtk::SelectionMode::None);
        flow_box.set_homogeneous(true);
        flow_box.set_min_children_per_line(3);
        flow_box.set_max_children_per_line(12);
        flow_box.set_row_spacing(8);
        flow_box.set_column_spacing(8);
        flow_box.set_margin_top(8);
        flow_box.set_margin_bottom(8);
        flow_box.set_margin_start(12);
        flow_box.set_margin_end(12);

        let separator = gtk::Separator::new(gtk::Orientation::Horizontal);

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.append(&separator);
        container.append(&flow_box);

        let revealer = gtk::Revealer::new();
        revealer.set_child(Some(&container));
        revealer.set_reveal_child(false);
        revealer.set_transition_type(gtk::RevealerTransitionType::SlideUp);
        revealer.set_transition_duration(200);

        ChordDiagramPanel {
            revealer,
            flow_box,
            chord_names: Rc::new(RefCell::new(Vec::new())),
            transpose_steps: Rc::new(Cell::new(0)),
        }
    }

    pub fn set_visible(&self, visible: bool) {
        self.revealer.set_reveal_child(visible);
    }

    /// Set the chord names found in the current tab (original, no transposition).
    /// These are the unique chord names like ["G", "D", "Em", "C"].
    pub fn set_chord_names(&self, names: &[String]) {
        *self.chord_names.borrow_mut() = names.to_vec();
        self.transpose_steps.set(0);
        self.rebuild();
    }

    /// Update transposition — transposes chord names and re-looks up voicings.
    pub fn set_transpose(&self, steps: i32) {
        self.transpose_steps.set(steps);
        self.rebuild();
    }

    pub fn clear(&self) {
        self.chord_names.borrow_mut().clear();
        self.clear_widgets();
    }

    fn rebuild(&self) {
        self.clear_widgets();
        let names = self.chord_names.borrow();
        let steps = self.transpose_steps.get();

        for name in names.iter() {
            let display_name = transpose_chord_name(name, steps);
            if let Some(voicing) = chord_db::get_voicing(&display_name) {
                let diagram = Self::create_diagram(&display_name, &voicing);
                self.flow_box.insert(&diagram, -1);
            }
            // If not in database, skip — don't show broken diagrams
        }
    }

    fn clear_widgets(&self) {
        while let Some(child) = self.flow_box.first_child() {
            self.flow_box.remove(&child);
        }
    }

    fn create_diagram(name: &str, voicing: &Voicing) -> gtk::Box {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 2);
        container.add_css_class("chord-diagram");

        let label = gtk::Label::new(Some(name));
        label.add_css_class("heading");
        container.append(&label);

        let drawing = gtk::DrawingArea::new();
        drawing.set_content_width(80);
        drawing.set_content_height(100);

        let voicing = *voicing;

        drawing.set_draw_func(move |_, cr, width, height| {
            Self::draw_diagram(cr, width as f64, height as f64, &voicing);
        });

        container.append(&drawing);
        container
    }

    fn draw_diagram(cr: &gtk::cairo::Context, w: f64, h: f64, voicing: &Voicing) {
        let num_strings = 6;
        let num_frets = 5;
        let margin_top = 14.0;
        let margin_bottom = 6.0;
        let margin_left = 12.0;
        let margin_right = 10.0;
        let string_spacing = (w - margin_left - margin_right) / (num_strings as f64 - 1.0);
        let fret_spacing = (h - margin_top - margin_bottom) / num_frets as f64;

        // Calculate fret offset (for chords high on the neck)
        let min_fret = voicing
            .iter()
            .filter_map(|f| f.filter(|&v| v > 0))
            .min()
            .unwrap_or(1);
        let max_fret = voicing
            .iter()
            .filter_map(|f| f.filter(|&v| v > 0))
            .max()
            .unwrap_or(1);
        let fret_offset = if max_fret > 5 { min_fret - 1 } else { 0 };

        // Colors
        let grid_color = (0.65, 0.65, 0.65, 1.0);
        let dot_color = (0.3, 0.3, 0.3, 1.0);
        let text_color = (0.55, 0.55, 0.55, 1.0);

        // Draw strings (vertical lines) — low E on LEFT, high E on RIGHT
        cr.set_source_rgba(grid_color.0, grid_color.1, grid_color.2, grid_color.3);
        for i in 0..num_strings {
            let x = margin_left + i as f64 * string_spacing;
            cr.move_to(x, margin_top);
            cr.line_to(x, h - margin_bottom);
        }
        cr.set_line_width(1.0);
        let _ = cr.stroke();

        // Draw frets (horizontal lines)
        for i in 0..=num_frets {
            let y = margin_top + i as f64 * fret_spacing;
            cr.move_to(margin_left, y);
            cr.line_to(margin_left + (num_strings - 1) as f64 * string_spacing, y);
        }
        cr.set_line_width(1.0);
        let _ = cr.stroke();

        // Nut (thick top line) if at position 0
        if fret_offset == 0 {
            cr.set_source_rgba(grid_color.0, grid_color.1, grid_color.2, grid_color.3);
            cr.set_line_width(3.0);
            cr.move_to(margin_left, margin_top);
            cr.line_to(
                margin_left + (num_strings - 1) as f64 * string_spacing,
                margin_top,
            );
            let _ = cr.stroke();
        }

        // Draw each string's state
        // voicing[0] = 6th string (low E) = leftmost on diagram
        // voicing[5] = 1st string (high E) = rightmost on diagram
        for (i, fret_opt) in voicing.iter().enumerate() {
            let x = margin_left + i as f64 * string_spacing;

            match fret_opt {
                None => {
                    // Muted: draw X above
                    cr.set_source_rgba(text_color.0, text_color.1, text_color.2, text_color.3);
                    cr.set_font_size(10.0);
                    if let Ok(extents) = cr.text_extents("x") {
                        cr.move_to(x - extents.width() / 2.0, margin_top - 3.0);
                        let _ = cr.show_text("x");
                    }
                }
                Some(0) => {
                    // Open: draw O above
                    cr.set_source_rgba(text_color.0, text_color.1, text_color.2, text_color.3);
                    cr.set_font_size(10.0);
                    if let Ok(extents) = cr.text_extents("o") {
                        cr.move_to(x - extents.width() / 2.0, margin_top - 3.0);
                        let _ = cr.show_text("o");
                    }
                }
                Some(fret) => {
                    // Fretted: draw filled circle
                    let display_fret = fret - fret_offset;
                    if display_fret >= 1 && display_fret <= num_frets as i32 {
                        let y = margin_top + (display_fret as f64 - 0.5) * fret_spacing;
                        cr.set_source_rgba(dot_color.0, dot_color.1, dot_color.2, dot_color.3);
                        cr.arc(x, y, 4.0, 0.0, 2.0 * PI);
                        let _ = cr.fill();
                    }
                }
            }
        }

        // Fret position indicator if offset > 0
        if fret_offset > 0 {
            cr.set_source_rgba(text_color.0, text_color.1, text_color.2, text_color.3);
            cr.set_font_size(9.0);
            let text = format!("{}fr", fret_offset + 1);
            cr.move_to(1.0, margin_top + fret_spacing * 0.6);
            let _ = cr.show_text(&text);
        }
    }
}

/// Transpose a full chord name like "Am7" or "F#m" by semitone steps.
fn transpose_chord_name(name: &str, steps: i32) -> String {
    if steps == 0 {
        return name.to_string();
    }
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return name.to_string();
    }
    let root_len = if bytes.len() >= 2 && (bytes[1] == b'#' || bytes[1] == b'b') {
        2
    } else {
        1
    };
    let root = &name[..root_len];
    let quality = &name[root_len..];
    format!("{}{}", transpose_note(root, steps), quality)
}
