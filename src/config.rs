use gtk4::gio;
use gtk4::prelude::*;

const APP_ID: &str = "de.chords.Chords";

/// Thin typed wrapper around GSettings.
#[derive(Clone)]
pub struct Settings {
    inner: gio::Settings,
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            inner: gio::Settings::new(APP_ID),
        }
    }

    // ── Window state ─────────────────────────

    pub fn window_width(&self) -> i32 {
        self.inner.int("window-width")
    }
    pub fn set_window_width(&self, v: i32) {
        let _ = self.inner.set_int("window-width", v);
    }

    pub fn window_height(&self) -> i32 {
        self.inner.int("window-height")
    }
    pub fn set_window_height(&self, v: i32) {
        let _ = self.inner.set_int("window-height", v);
    }

    pub fn is_maximized(&self) -> bool {
        self.inner.boolean("is-maximized")
    }
    pub fn set_is_maximized(&self, v: bool) {
        let _ = self.inner.set_boolean("is-maximized", v);
    }

    // ── Font ─────────────────────────────────

    pub fn font_family(&self) -> String {
        self.inner.string("font-family").to_string()
    }
    pub fn set_font_family(&self, v: &str) {
        let _ = self.inner.set_string("font-family", v);
    }

    pub fn font_size(&self) -> i32 {
        self.inner.int("font-size")
    }
    pub fn set_font_size(&self, v: i32) {
        let _ = self.inner.set_int("font-size", v);
    }

    // ── Colors ───────────────────────────────

    pub fn chord_color(&self) -> String {
        self.inner.string("chord-color").to_string()
    }
    pub fn set_chord_color(&self, v: &str) {
        let _ = self.inner.set_string("chord-color", v);
    }

    pub fn section_color(&self) -> String {
        self.inner.string("section-color").to_string()
    }
    pub fn set_section_color(&self, v: &str) {
        let _ = self.inner.set_string("section-color", v);
    }

    // ── View ─────────────────────────────────

    pub fn columns(&self) -> i32 {
        self.inner.int("columns")
    }
    pub fn set_columns(&self, v: i32) {
        let _ = self.inner.set_int("columns", v);
    }

    pub fn show_chord_diagrams(&self) -> bool {
        self.inner.boolean("show-chord-diagrams")
    }
    pub fn set_show_chord_diagrams(&self, v: bool) {
        let _ = self.inner.set_boolean("show-chord-diagrams", v);
    }
}
