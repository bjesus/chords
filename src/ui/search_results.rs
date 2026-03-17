use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{self as gtk};

use crate::data::models::SearchResult;

/// Search results view displayed in the content area.
#[derive(Clone)]
pub struct SearchResultsView {
    pub scrolled_window: gtk::ScrolledWindow,
    pub list_box: gtk::ListBox,
    activated_callbacks: Rc<RefCell<Vec<Box<dyn Fn(String)>>>>,
}

impl SearchResultsView {
    pub fn new() -> Self {
        let list_box = gtk::ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::Single);
        list_box.add_css_class("boxed-list");
        list_box.set_margin_top(12);
        list_box.set_margin_bottom(12);
        list_box.set_margin_start(12);
        list_box.set_margin_end(12);

        let clamp = libadwaita::Clamp::new();
        clamp.set_maximum_size(800);
        clamp.set_child(Some(&list_box));

        let scrolled_window = gtk::ScrolledWindow::new();
        scrolled_window.set_vexpand(true);
        scrolled_window.set_child(Some(&clamp));

        let activated_callbacks: Rc<RefCell<Vec<Box<dyn Fn(String)>>>> =
            Rc::new(RefCell::new(Vec::new()));

        // Connect row activation
        {
            let cbs = Rc::clone(&activated_callbacks);
            list_box.connect_row_activated(move |_, row| {
                if let Some(url) = row.widget_name().strip_prefix("result:") {
                    let url = url.to_string();
                    for cb in cbs.borrow().iter() {
                        cb(url.clone());
                    }
                }
            });
        }

        SearchResultsView {
            scrolled_window,
            list_box,
            activated_callbacks,
        }
    }

    pub fn connect_activated<F: Fn(String) + 'static>(&self, f: F) {
        self.activated_callbacks.borrow_mut().push(Box::new(f));
    }

    /// Replace current results with new ones.
    pub fn set_results(&self, results: &[SearchResult]) {
        // Clear existing
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        for result in results {
            let row = Self::create_row(result);
            self.list_box.append(&row);
        }
    }

    fn create_row(result: &SearchResult) -> gtk::ListBoxRow {
        let title_str = if result.version > 1 {
            format!("{} (ver {})", result.song_name, result.version)
        } else {
            result.song_name.clone()
        };

        // Rating display
        let rating_str = if result.votes > 0 {
            format!("{}/5 ({})", result.rating, result.votes)
        } else {
            String::new()
        };

        // Type badge
        let type_str = result.tab_type.display_name();

        // Title
        let title_label = gtk::Label::new(Some(&title_str));
        title_label.set_xalign(0.0);
        title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title_label.set_hexpand(true);
        title_label.add_css_class("heading");

        // Artist
        let artist_label = gtk::Label::new(Some(&result.artist_name));
        artist_label.set_xalign(0.0);
        artist_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        artist_label.add_css_class("dim-label");

        // Info row: type + rating
        let type_label = gtk::Label::new(Some(type_str));
        type_label.add_css_class("caption");
        type_label.add_css_class("dim-label");

        let info_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        info_box.append(&type_label);

        if !rating_str.is_empty() {
            let sep = gtk::Label::new(Some("·"));
            sep.add_css_class("dim-label");
            let rating_label = gtk::Label::new(Some(&rating_str));
            rating_label.add_css_class("caption");
            rating_label.add_css_class("dim-label");
            info_box.append(&sep);
            info_box.append(&rating_label);
        }

        // Left side (title + artist)
        let left_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        left_box.set_hexpand(true);
        left_box.append(&title_label);
        left_box.append(&artist_label);
        left_box.append(&info_box);

        // Row
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(8);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);
        hbox.append(&left_box);

        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&hbox));
        row.set_widget_name(&format!("result:{}", result.tab_url));
        row.add_css_class("search-result-row");

        row
    }
}
