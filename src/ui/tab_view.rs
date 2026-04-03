use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{self as gtk, glib};

use crate::data::models::{LineKind, ParsedLine};

/// The main tab/chord content renderer using GtkTextView.
/// Columns are implemented by splitting content across multiple TextViews
/// arranged horizontally, since GTK4 CSS does not support column-count.
#[derive(Clone)]
pub struct TabView {
    pub scrolled_window: gtk::ScrolledWindow,
    columns_box: gtk::Box,
    text_views: Rc<std::cell::RefCell<Vec<gtk::TextView>>>,
    buffers: Rc<std::cell::RefCell<Vec<gtk::TextBuffer>>>,
    lines_cache: Rc<std::cell::RefCell<Vec<ParsedLine>>>,
    font_family: Rc<std::cell::RefCell<String>>,
    font_size: Rc<Cell<i32>>,
    chord_color: Rc<std::cell::RefCell<String>>,
    section_color: Rc<std::cell::RefCell<String>>,
    column_count: Rc<Cell<i32>>,
    scroll_speed_ms: Rc<Cell<u32>>,
    scroll_source_id: Rc<std::cell::RefCell<Option<glib::SourceId>>>,
    inhibit_cookie: Rc<Cell<Option<u32>>>,
    css_provider: gtk::CssProvider,
}

impl TabView {
    pub fn new() -> Self {
        let columns_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        columns_box.set_hexpand(true);

        // Start with a single text view
        let buffer = gtk::TextBuffer::new(None);
        Self::create_tags(&buffer, "#3584e4", "#e66100");
        let text_view = Self::make_text_view(&buffer);

        columns_box.append(&text_view);

        let scrolled_window = gtk::ScrolledWindow::new();
        scrolled_window.set_vexpand(true);
        scrolled_window.set_hexpand(true);
        scrolled_window.set_child(Some(&columns_box));

        let css_provider = gtk::CssProvider::new();

        let tv = TabView {
            scrolled_window,
            columns_box,
            text_views: Rc::new(std::cell::RefCell::new(vec![text_view])),
            buffers: Rc::new(std::cell::RefCell::new(vec![buffer])),
            lines_cache: Rc::new(std::cell::RefCell::new(Vec::new())),
            font_family: Rc::new(std::cell::RefCell::new("Monospace".to_string())),
            font_size: Rc::new(Cell::new(14)),
            chord_color: Rc::new(std::cell::RefCell::new("#3584e4".to_string())),
            section_color: Rc::new(std::cell::RefCell::new("#e66100".to_string())),
            column_count: Rc::new(Cell::new(1)),
            scroll_speed_ms: Rc::new(Cell::new(500)),
            scroll_source_id: Rc::new(std::cell::RefCell::new(None)),
            inhibit_cookie: Rc::new(Cell::new(None)),
            css_provider,
        };

        tv.apply_font();
        tv
    }

    fn make_text_view(buffer: &gtk::TextBuffer) -> gtk::TextView {
        let text_view = gtk::TextView::with_buffer(buffer);
        text_view.set_editable(false);
        text_view.set_cursor_visible(false);
        text_view.set_wrap_mode(gtk::WrapMode::None);
        text_view.set_monospace(true);
        text_view.set_left_margin(16);
        text_view.set_right_margin(16);
        text_view.set_top_margin(12);
        text_view.set_bottom_margin(12);
        text_view.set_hexpand(true);
        text_view.set_vexpand(true);
        text_view.set_extra_menu(Some(&gtk::gio::Menu::new()));
        text_view
    }

    fn create_tags(buffer: &gtk::TextBuffer, chord_color: &str, section_color: &str) {
        let tag_table = buffer.tag_table();

        let chord_tag = gtk::TextTag::new(Some("chord"));
        chord_tag.set_weight(700);
        chord_tag.set_foreground(Some(chord_color));
        tag_table.add(&chord_tag);

        let section_tag = gtk::TextTag::new(Some("section"));
        section_tag.set_weight(700);
        section_tag.set_foreground(Some(section_color));
        section_tag.set_pixels_above_lines(8);
        tag_table.add(&section_tag);
    }

    /// Render parsed lines, splitting across columns if needed.
    pub fn render_lines(&self, lines: &[ParsedLine]) {
        *self.lines_cache.borrow_mut() = lines.to_vec();
        self.rebuild_columns();
    }

    fn rebuild_columns(&self) {
        let cols = self.column_count.get().max(1) as usize;
        let lines = self.lines_cache.borrow();

        // Remove all existing text views from columns_box
        while let Some(child) = self.columns_box.first_child() {
            self.columns_box.remove(&child);
        }

        // Split lines into N roughly equal chunks
        let chunks = Self::split_lines(&lines, cols);

        let mut new_views = Vec::new();
        let mut new_buffers = Vec::new();

        let cc = self.chord_color.borrow().clone();
        let sc = self.section_color.borrow().clone();

        for (i, chunk) in chunks.iter().enumerate() {
            let buffer = gtk::TextBuffer::new(None);
            Self::create_tags(&buffer, &cc, &sc);
            let text_view = Self::make_text_view(&buffer);

            Self::render_lines_into_buffer(&buffer, chunk);

            // Add a separator between columns
            if i > 0 {
                let sep = gtk::Separator::new(gtk::Orientation::Vertical);
                self.columns_box.append(&sep);
            }
            self.columns_box.append(&text_view);

            new_views.push(text_view);
            new_buffers.push(buffer);
        }

        *self.text_views.borrow_mut() = new_views;
        *self.buffers.borrow_mut() = new_buffers;

        // Re-apply font to all new text views
        self.apply_font();
    }

    fn split_lines(lines: &[ParsedLine], cols: usize) -> Vec<Vec<ParsedLine>> {
        if cols <= 1 || lines.is_empty() {
            return vec![lines.to_vec()];
        }

        let total = lines.len();
        let chunk_size = (total + cols - 1) / cols;
        lines.chunks(chunk_size).map(|c| c.to_vec()).collect()
    }

    fn render_lines_into_buffer(buffer: &gtk::TextBuffer, lines: &[ParsedLine]) {
        buffer.set_text("");
        let mut iter = buffer.start_iter();

        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                buffer.insert(&mut iter, "\n");
            }

            match line.kind {
                LineKind::Empty => {}
                LineKind::SectionHeader => {
                    let start_offset = iter.offset();
                    buffer.insert(&mut iter, &line.content);
                    let start = buffer.iter_at_offset(start_offset);
                    buffer.apply_tag_by_name("section", &start, &iter);
                }
                LineKind::ChordLine | LineKind::LyricLine => {
                    if line.chords.is_empty() {
                        buffer.insert(&mut iter, &line.content);
                    } else {
                        Self::render_chord_line_static(buffer, &mut iter, line);
                    }
                }
            }
        }
    }

    fn render_chord_line_static(
        buffer: &gtk::TextBuffer,
        iter: &mut gtk::TextIter,
        line: &ParsedLine,
    ) {
        let content = &line.content;
        let mut last_end = 0;

        for chord in &line.chords {
            if chord.column > last_end && chord.column <= content.len() {
                buffer.insert(iter, &content[last_end..chord.column]);
            }

            let chord_text = chord.display();
            let start_offset = iter.offset();
            buffer.insert(iter, &chord_text);
            let start = buffer.iter_at_offset(start_offset);
            buffer.apply_tag_by_name("chord", &start, iter);

            last_end = chord.column + chord_text.len();
        }

        if last_end < content.len() {
            buffer.insert(iter, &content[last_end..]);
        }
    }

    pub fn set_font_size(&self, size: i32) {
        self.font_size.set(size.clamp(8, 32));
        self.apply_font();
    }

    pub fn get_font_size(&self) -> i32 {
        self.font_size.get()
    }

    pub fn set_font_family(&self, family: &str) {
        *self.font_family.borrow_mut() = family.to_string();
        self.apply_font();
    }

    fn apply_font(&self) {
        let family = self.font_family.borrow().clone();
        let size = self.font_size.get();

        let css = format!(
            "textview {{ font-family: \"{}\"; font-size: {}pt; }}",
            family, size
        );

        self.css_provider.load_from_data(&css);

        // Apply to all text views
        for tv in self.text_views.borrow().iter() {
            gtk::style_context_add_provider_for_display(
                &tv.display(),
                &self.css_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }
    }

    pub fn set_chord_color(&self, color: &str) {
        *self.chord_color.borrow_mut() = color.to_string();
        self.rebuild_columns(); // re-render with new color
    }

    pub fn set_section_color(&self, color: &str) {
        *self.section_color.borrow_mut() = color.to_string();
        self.rebuild_columns();
    }

    // --- Column support ---
    pub fn set_columns(&self, cols: i32) {
        let clamped = cols.clamp(1, 4);
        if clamped != self.column_count.get() {
            self.column_count.set(clamped);
            self.rebuild_columns();
        }
    }

    // --- Auto-scroll ---
    pub fn start_autoscroll(&self, window: &gtk4::Window) {
        self.stop_autoscroll(window);

        // Don't start if there's nothing to scroll
        let vadj = self.scrolled_window.vadjustment();
        if vadj.upper() <= vadj.page_size() {
            return;
        }

        // Inhibit idle/screensaver while scrolling
        if let Some(app) = window.application() {
            let cookie = app.inhibit(
                Some(window),
                gtk::ApplicationInhibitFlags::IDLE,
                Some("Auto-scroll is active"),
            );
            self.inhibit_cookie.set(Some(cookie));
        }

        let scrolled = self.scrolled_window.clone();
        let speed = self.scroll_speed_ms.get();
        let source_id_ref = self.scroll_source_id.clone();

        let source_id =
            glib::timeout_add_local(std::time::Duration::from_millis(speed as u64), move || {
                let vadj = scrolled.vadjustment();
                let current = vadj.value();
                let upper = vadj.upper() - vadj.page_size();
                if current < upper {
                    vadj.set_value(current + 1.0);
                    glib::ControlFlow::Continue
                } else {
                    // Timer ended naturally — clear stored ID so stop_autoscroll
                    // won't try to remove an already-dead source
                    *source_id_ref.borrow_mut() = None;
                    glib::ControlFlow::Break
                }
            });

        *self.scroll_source_id.borrow_mut() = Some(source_id);
    }

    pub fn stop_autoscroll(&self, window: &gtk4::Window) {
        if let Some(source_id) = self.scroll_source_id.borrow_mut().take() {
            source_id.remove();
        }
        // Release the screen inhibition
        if let Some(cookie) = self.inhibit_cookie.take() {
            if let Some(app) = window.application() {
                app.uninhibit(cookie);
            }
        }
    }

    pub fn set_scroll_speed(&self, ms: u32, window: &gtk4::Window) {
        self.scroll_speed_ms.set(ms.max(10));
        if self.scroll_source_id.borrow().is_some() {
            self.start_autoscroll(window);
        }
    }

    pub fn is_scrolling(&self) -> bool {
        self.scroll_source_id.borrow().is_some()
    }
}
