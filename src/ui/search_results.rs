use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{self as gtk};
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::data::models::{SearchResult, SongGroup};

/// Search results view with songs grouped by name, each with an expandable
/// list of alternative versions.
#[derive(Clone)]
pub struct SearchResultsView {
    pub scrolled_window: gtk::ScrolledWindow,
    list_box: gtk::ListBox,
    loading_row: gtk::ListBoxRow,
    activated_callbacks: Rc<RefCell<Vec<Box<dyn Fn(String)>>>>,
}

impl SearchResultsView {
    pub fn new() -> Self {
        let list_box = gtk::ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::None);
        list_box.add_css_class("boxed-list");
        list_box.set_margin_top(12);
        list_box.set_margin_bottom(12);
        list_box.set_margin_start(12);
        list_box.set_margin_end(12);

        // "Loading more…" footer row (hidden by default)
        let loading_spinner = gtk::Spinner::new();
        loading_spinner.set_spinning(true);
        let loading_label = gtk::Label::new(Some("Loading more…"));
        loading_label.add_css_class("dim-label");
        let loading_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        loading_box.set_halign(gtk::Align::Center);
        loading_box.set_margin_top(10);
        loading_box.set_margin_bottom(10);
        loading_box.append(&loading_spinner);
        loading_box.append(&loading_label);

        let loading_row = gtk::ListBoxRow::new();
        loading_row.set_child(Some(&loading_box));
        loading_row.set_activatable(false);
        loading_row.set_selectable(false);
        loading_row.set_visible(false);
        list_box.append(&loading_row);

        let clamp = adw::Clamp::new();
        clamp.set_maximum_size(800);
        clamp.set_child(Some(&list_box));

        let scrolled_window = gtk::ScrolledWindow::new();
        scrolled_window.set_vexpand(true);
        scrolled_window.set_child(Some(&clamp));

        SearchResultsView {
            scrolled_window,
            list_box,
            loading_row,
            activated_callbacks: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn connect_activated<F: Fn(String) + 'static>(&self, f: F) {
        self.activated_callbacks.borrow_mut().push(Box::new(f));
    }

    /// Clear all results.
    pub fn clear(&self) {
        while let Some(child) = self.list_box.first_child() {
            // Don't remove the loading row
            if child == self.loading_row.clone().upcast::<gtk::Widget>() {
                break;
            }
            self.list_box.remove(&child);
        }
    }

    /// Render a fresh set of groups (page 1 — clears existing).
    pub fn set_groups(&self, groups: &[SongGroup]) {
        self.clear();
        for group in groups {
            let row = self.create_group_row(group);
            self.list_box.insert(&row, -1);
        }
        // Keep loading row at the end
        self.list_box.remove(&self.loading_row);
        self.list_box.append(&self.loading_row);
    }

    /// Update the full groups list in place (subsequent pages).
    /// Adds new groups and refreshes existing ones with more versions.
    pub fn update_groups(&self, groups: &[SongGroup]) {
        // Simple approach: rebuild entirely. The list isn't that long
        // and rebuilding avoids complex diffing.
        self.set_groups(groups);
    }

    pub fn set_loading_more(&self, loading: bool) {
        self.loading_row.set_visible(loading);
        if let Some(child) = self.loading_row.child() {
            if let Some(b) = child.downcast_ref::<gtk::Box>() {
                if let Some(spinner) = b.first_child()
                    .and_then(|w| w.downcast::<gtk::Spinner>().ok())
                {
                    if loading { spinner.start(); } else { spinner.stop(); }
                }
            }
        }
    }

    fn create_group_row(&self, group: &SongGroup) -> gtk::Widget {
        let best = group.best();
        let callbacks = Rc::clone(&self.activated_callbacks);
        let best_url = best.tab_url.clone();

        if group.versions.len() == 1 {
            // Single version — plain ActionRow
            let row = adw::ActionRow::new();
            row.set_title(&group.song_name);
            row.set_subtitle(&group.artist_name);
            row.add_css_class("search-result-row");

            let suffix = Self::version_suffix(best);
            row.add_suffix(&suffix);
            row.set_activatable(true);

            row.connect_activated(move |_| {
                for cb in callbacks.borrow().iter() {
                    cb(best_url.clone());
                }
            });

            row.upcast()
        } else {
            // Multiple versions — ExpanderRow with best version as primary action
            let expander = adw::ExpanderRow::new();
            expander.set_title(&group.song_name);
            expander.set_subtitle(&group.artist_name);
            expander.add_css_class("search-result-row");

            // Best version suffix + an open button (both in the suffix area)
            let best_suffix = Self::version_suffix(best);
            expander.add_suffix(&best_suffix);

            // Open button in suffix — opens best version without expanding
            let open_btn = gtk::Button::from_icon_name("media-playback-start-symbolic");
            open_btn.set_tooltip_text(Some("Open best version"));
            open_btn.add_css_class("flat");
            open_btn.set_valign(gtk::Align::Center);
            expander.add_suffix(&open_btn);

            {
                let cbs = Rc::clone(&self.activated_callbacks);
                let url = best_url.clone();
                open_btn.connect_clicked(move |_| {
                    for cb in cbs.borrow().iter() { cb(url.clone()); }
                });
            }

            // Sub-rows for each alternative version
            let other_count = group.versions.len() - 1;
            let count_label_text = format!(
                "{} other version{}",
                other_count,
                if other_count == 1 { "" } else { "s" }
            );
            expander.set_subtitle(&format!(
                "{} · {}",
                group.artist_name, count_label_text
            ));

            for version in group.versions.iter() {
                let sub = self.create_version_subrow(version);
                expander.add_row(&sub);
            }

            expander.upcast()
        }
    }

    fn create_version_subrow(&self, result: &SearchResult) -> adw::ActionRow {
        let row = adw::ActionRow::new();

        let title = if result.version > 1 {
            format!("{} ver {}", result.tab_type.display_name(), result.version)
        } else {
            result.tab_type.display_name().to_string()
        };
        row.set_title(&title);

        let suffix = Self::version_suffix(result);
        row.add_suffix(&suffix);
        row.set_activatable(true);

        let url = result.tab_url.clone();
        let callbacks = Rc::clone(&self.activated_callbacks);
        row.connect_activated(move |_| {
            for cb in callbacks.borrow().iter() {
                cb(url.clone());
            }
        });

        row
    }

    fn version_suffix(result: &SearchResult) -> gtk::Box {
        let b = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        b.set_valign(gtk::Align::Center);

        if result.votes > 0 {
            let rating = gtk::Label::new(Some(&format!("{}", result.rating)));
            rating.add_css_class("dim-label");
            rating.add_css_class("caption");
            b.append(&rating);
        }

        let type_label = gtk::Label::new(Some(result.tab_type.display_name()));
        type_label.add_css_class("caption");
        type_label.add_css_class("dim-label");
        b.append(&type_label);

        b
    }
}
