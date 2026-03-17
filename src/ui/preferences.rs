use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{self as gtk, pango};
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::config::Settings;

/// Callbacks that persist across preferences dialog open/close cycles.
#[derive(Clone)]
struct Callbacks {
    font_family: Rc<RefCell<Vec<Box<dyn Fn(String)>>>>,
    font_size: Rc<RefCell<Vec<Box<dyn Fn(i32)>>>>,
    chord_color: Rc<RefCell<Vec<Box<dyn Fn(String)>>>>,
    section_color: Rc<RefCell<Vec<Box<dyn Fn(String)>>>>,
}

/// Preferences manager — uses AdwPreferencesDialog (libadwaita 1.5+).
/// Creates a fresh dialog each time since AdwDialog handles its own lifecycle.
#[derive(Clone)]
pub struct PreferencesManager {
    cbs: Callbacks,
}

impl PreferencesManager {
    pub fn new() -> Self {
        PreferencesManager {
            cbs: Callbacks {
                font_family: Rc::new(RefCell::new(Vec::new())),
                font_size: Rc::new(RefCell::new(Vec::new())),
                chord_color: Rc::new(RefCell::new(Vec::new())),
                section_color: Rc::new(RefCell::new(Vec::new())),
            },
        }
    }

    /// Present a fresh preferences dialog over the parent window.
    pub fn present(&self, parent: &adw::ApplicationWindow, settings: &Settings) {
        let dialog = adw::PreferencesDialog::new();

        let page = adw::PreferencesPage::new();
        page.set_title("Appearance");
        page.set_icon_name(Some("applications-graphics-symbolic"));

        // ── Font ─────────────────────────────────
        let font_group = adw::PreferencesGroup::new();
        font_group.set_title("Font");

        // Font family: native GTK font dialog, filtered to monospace
        let font_dialog = gtk::FontDialog::new();
        let mono_filter = gtk::CustomFilter::new(|item| {
            item.downcast_ref::<pango::FontFamily>()
                .map_or(false, |f| f.is_monospace())
        });
        font_dialog.set_filter(Some(&mono_filter));

        let font_btn = gtk::FontDialogButton::new(Some(font_dialog));
        font_btn.set_level(gtk::FontLevel::Family);
        font_btn.set_font_desc(&pango::FontDescription::from_string(&settings.font_family()));
        font_btn.set_valign(gtk::Align::Center);

        let font_row = adw::ActionRow::new();
        font_row.set_title("Font Family");
        font_row.add_suffix(&font_btn);
        font_row.set_activatable_widget(Some(&font_btn));

        let size_adj = gtk::Adjustment::new(settings.font_size() as f64, 8.0, 32.0, 1.0, 2.0, 0.0);
        let size_row = adw::SpinRow::new(Some(&size_adj), 1.0, 0);
        size_row.set_title("Font Size");

        font_group.add(&font_row);
        font_group.add(&size_row);
        page.add(&font_group);

        // ── Colors ───────────────────────────────
        let colors_group = adw::PreferencesGroup::new();
        colors_group.set_title("Highlight Colors");

        let chord_color_btn = Self::make_color_button(&settings.chord_color());
        let chord_row = adw::ActionRow::new();
        chord_row.set_title("Chord Color");
        chord_row.add_suffix(&chord_color_btn);
        chord_row.set_activatable_widget(Some(&chord_color_btn));

        let section_color_btn = Self::make_color_button(&settings.section_color());
        let section_row = adw::ActionRow::new();
        section_row.set_title("Section Header Color");
        section_row.add_suffix(&section_color_btn);
        section_row.set_activatable_widget(Some(&section_color_btn));

        colors_group.add(&chord_row);
        colors_group.add(&section_row);
        page.add(&colors_group);

        dialog.add(&page);

        // ── Wire signals ─────────────────────────
        {
            let cbs = Rc::clone(&self.cbs.font_family);
            font_btn.connect_font_desc_notify(move |btn: &gtk::FontDialogButton| {
                if let Some(desc) = btn.font_desc() {
                    if let Some(family) = desc.family() {
                        let name = family.to_string();
                        for cb in cbs.borrow().iter() { cb(name.clone()); }
                    }
                }
            });
        }
        {
            let cbs = Rc::clone(&self.cbs.font_size);
            size_row.connect_changed(move |row| {
                let size = row.value() as i32;
                for cb in cbs.borrow().iter() { cb(size); }
            });
        }
        {
            let cbs = Rc::clone(&self.cbs.chord_color);
            chord_color_btn.connect_rgba_notify(move |btn: &gtk::ColorDialogButton| {
                let hex = rgba_to_hex(&btn.rgba());
                for cb in cbs.borrow().iter() { cb(hex.clone()); }
            });
        }
        {
            let cbs = Rc::clone(&self.cbs.section_color);
            section_color_btn.connect_rgba_notify(move |btn: &gtk::ColorDialogButton| {
                let hex = rgba_to_hex(&btn.rgba());
                for cb in cbs.borrow().iter() { cb(hex.clone()); }
            });
        }

        dialog.present(Some(parent));
    }

    fn make_color_button(hex: &str) -> gtk::ColorDialogButton {
        let d = gtk::ColorDialog::new();
        d.set_with_alpha(false);
        let btn = gtk::ColorDialogButton::new(Some(d));
        if let Ok(rgba) = gtk::gdk::RGBA::parse(hex) {
            btn.set_rgba(&rgba);
        }
        btn.set_valign(gtk::Align::Center);
        btn
    }

    // ── Callback connectors ──────────────────
    pub fn connect_font_family_changed<F: Fn(String) + 'static>(&self, f: F) {
        self.cbs.font_family.borrow_mut().push(Box::new(f));
    }
    pub fn connect_font_size_changed<F: Fn(i32) + 'static>(&self, f: F) {
        self.cbs.font_size.borrow_mut().push(Box::new(f));
    }
    pub fn connect_chord_color_changed<F: Fn(String) + 'static>(&self, f: F) {
        self.cbs.chord_color.borrow_mut().push(Box::new(f));
    }
    pub fn connect_section_color_changed<F: Fn(String) + 'static>(&self, f: F) {
        self.cbs.section_color.borrow_mut().push(Box::new(f));
    }
}

fn rgba_to_hex(rgba: &gtk::gdk::RGBA) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (rgba.red() * 255.0) as u8,
        (rgba.green() * 255.0) as u8,
        (rgba.blue() * 255.0) as u8,
    )
}
