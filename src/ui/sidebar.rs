use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

use gtk4::prelude::*;
use gtk4::{self as gtk, glib};
use libadwaita as adw;

use crate::data::cache::Cache;
use crate::data::models::SavedTabSummary;

/// The sidebar showing saved chords, grouped by artist with avatars.
/// Includes a filter entry (Ctrl+K) that filters the list in real time.
#[derive(Clone)]
pub struct Sidebar {
    pub container: gtk::Box,
    pub list_box: gtk::ListBox,
    pub filter_entry: gtk::SearchEntry,
    filter_bar: gtk::SearchBar,
    stack: gtk::Stack,
    activated_callbacks: Rc<RefCell<Vec<Box<dyn Fn(String)>>>>,
    filter_text: Rc<RefCell<String>>,
}

impl Sidebar {
    pub fn new(cache: &Arc<Cache>) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);

        // Sidebar header
        let sidebar_header = adw::HeaderBar::new();
        sidebar_header.set_show_title(true);
        let sidebar_title = adw::WindowTitle::new("Library", "");
        sidebar_header.set_title_widget(Some(&sidebar_title));
        container.append(&sidebar_header);

        // Filter entry in a SearchBar (slides in on Ctrl+K)
        let filter_entry = gtk::SearchEntry::new();
        filter_entry.set_hexpand(true);
        filter_entry.set_placeholder_text(Some("Filter library (Ctrl+K)"));

        let filter_bar = gtk::SearchBar::new();
        filter_bar.set_child(Some(&filter_entry));
        filter_bar.set_search_mode(false);
        filter_bar.connect_entry(&filter_entry);
        container.append(&filter_bar);

        // List box
        let list_box = gtk::ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::Single);
        list_box.add_css_class("navigation-sidebar");

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_child(Some(&list_box));

        let empty_status = adw::StatusPage::new();
        empty_status.set_icon_name(Some("starred-symbolic"));
        empty_status.set_title("No Saved Songs");
        empty_status.set_description(Some("Songs you save will appear here"));

        let stack = gtk::Stack::new();
        stack.add_named(&scrolled, Some("list"));
        stack.add_named(&empty_status, Some("empty"));

        container.append(&stack);

        let filter_text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        let sidebar = Sidebar {
            container,
            list_box,
            filter_entry: filter_entry.clone(),
            filter_bar: filter_bar.clone(),
            stack,
            activated_callbacks: Rc::new(RefCell::new(Vec::new())),
            filter_text: filter_text.clone(),
        };

        sidebar.refresh(cache);

        // Row activation — only song rows (prefixed "tab:")
        {
            let callbacks = Rc::clone(&sidebar.activated_callbacks);
            sidebar.list_box.connect_row_activated(move |_, row| {
                if let Some(url) = row.widget_name().strip_prefix("tab:") {
                    let url = url.to_string();
                    for cb in callbacks.borrow().iter() {
                        cb(url.clone());
                    }
                }
            });
        }

        // Filter: update on every keystroke
        {
            let ft = filter_text.clone();
            let lb = sidebar.list_box.clone();
            filter_entry.connect_search_changed(move |entry| {
                let text = entry.text().to_string().to_lowercase();
                *ft.borrow_mut() = text;
                lb.invalidate_filter();
            });
        }

        // Set filter function on list box
        {
            let ft = filter_text.clone();
            sidebar.list_box.set_filter_func(move |row| {
                let filter = ft.borrow();
                if filter.is_empty() {
                    return true; // show all when no filter
                }

                let name = row.widget_name();

                // Artist header rows have no "tab:" prefix — show them if any
                // of their children would match. We use a simpler approach:
                // show header rows always, hide song rows that don't match.
                if !name.starts_with("tab:") {
                    // This is an artist header. Check if the artist name matches.
                    // The artist name is in the row's child label.
                    if let Some(hbox) = row.child() {
                        if let Some(label) = hbox.last_child() {
                            if let Some(label) = label.downcast_ref::<gtk::Label>() {
                                let artist = label.text().to_string().to_lowercase();
                                if artist.contains(filter.as_str()) {
                                    return true;
                                }
                            }
                        }
                    }
                    // Show artist header if any song row after it could match
                    // We can't easily look ahead, so always show headers
                    // and rely on the visual being fine with some empty groups
                    return true;
                }

                // Song row: check the song name label
                if let Some(label) = row.child() {
                    if let Some(label) = label.downcast_ref::<gtk::Label>() {
                        let song = label.text().to_string().to_lowercase();
                        if song.contains(filter.as_str()) {
                            return true;
                        }
                    }
                }

                // Also match against the full widget name which contains the URL
                // (which has the artist slug and song slug)
                name.to_lowercase().contains(filter.as_str())
            });
        }

        // Enter in filter → activate the first matching song row
        {
            let lb = sidebar.list_box.clone();
            let fb = filter_bar.clone();
            let ft = sidebar.filter_text.clone();
            let callbacks = Rc::clone(&sidebar.activated_callbacks);
            filter_entry.connect_activate(move |_| {
                let filter = ft.borrow().clone();
                let mut idx = 0;
                while let Some(row) = lb.row_at_index(idx) {
                    let name = row.widget_name();
                    if name.starts_with("tab:") {
                        // Check if this row matches the filter (same logic as filter_func)
                        let matches = if filter.is_empty() {
                            true
                        } else {
                            // Check song label text
                            let label_match = row.child()
                                .and_then(|w| w.downcast::<gtk::Label>().ok())
                                .map_or(false, |l| {
                                    l.text().to_string().to_lowercase().contains(&filter)
                                });
                            // Check URL slug
                            label_match || name.to_lowercase().contains(&filter)
                        };

                        if matches {
                            let url = name.strip_prefix("tab:").unwrap().to_string();
                            for cb in callbacks.borrow().iter() {
                                cb(url.clone());
                            }
                            fb.set_search_mode(false);
                            break;
                        }
                    }
                    idx += 1;
                }
            });
        }

        // When filter bar is dismissed, clear the filter
        {
            let fe = filter_entry.clone();
            let lb = sidebar.list_box.clone();
            let ft = sidebar.filter_text.clone();
            filter_bar.connect_search_mode_enabled_notify(move |bar| {
                if !bar.is_search_mode() {
                    fe.set_text("");
                    *ft.borrow_mut() = String::new();
                    lb.invalidate_filter();
                }
            });
        }

        sidebar
    }

    /// Show the filter bar and focus the entry.
    pub fn show_filter(&self) {
        self.filter_bar.set_search_mode(true);
        self.filter_entry.grab_focus();
    }

    pub fn connect_activated<F: Fn(String) + 'static>(&self, f: F) {
        self.activated_callbacks.borrow_mut().push(Box::new(f));
    }

    /// Refresh the sidebar list from the cache, grouped by artist A-Z.
    pub fn refresh(&self, cache: &Arc<Cache>) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        match cache.list_saved_tabs() {
            Ok(tabs) => {
                if tabs.is_empty() {
                    self.stack.set_visible_child_name("empty");
                    return;
                }

                self.stack.set_visible_child_name("list");

                let mut by_artist: BTreeMap<String, Vec<&SavedTabSummary>> = BTreeMap::new();
                for tab in &tabs {
                    by_artist
                        .entry(tab.artist_name.clone())
                        .or_default()
                        .push(tab);
                }

                for songs in by_artist.values_mut() {
                    songs.sort_by(|a, b| a.song_name.cmp(&b.song_name));
                }

                for (artist, songs) in &by_artist {
                    let header_row = Self::create_artist_header(artist, cache);
                    self.list_box.append(&header_row);

                    for tab in songs {
                        let row = Self::create_song_row(tab);
                        self.list_box.append(&row);
                    }
                }

                // Re-apply filter if active
                self.list_box.invalidate_filter();
            }
            Err(e) => {
                eprintln!("Failed to load saved tabs: {}", e);
            }
        }
    }

    fn create_artist_header(artist: &str, cache: &Arc<Cache>) -> gtk::ListBoxRow {
        let avatar = adw::Avatar::new(24, Some(artist), false);

        if let Some(image_data) = cache.get_artist_image(artist) {
            if let Some(texture) = Self::bytes_to_texture(&image_data) {
                let paintable = gtk::gdk::Paintable::from(texture);
                avatar.set_custom_image(Some(&paintable));
            }
        } else {
            let artist_name = artist.to_string();
            let cache = Arc::clone(cache);
            let avatar_clone = avatar.clone();
            glib::spawn_future_local(async move {
                match crate::data::api::fetch_artist_image(&artist_name).await {
                    Ok(data) => {
                        cache.save_artist_image(&artist_name, &data);
                        if let Some(texture) = Self::bytes_to_texture(&data) {
                            let paintable = gtk::gdk::Paintable::from(texture);
                            avatar_clone.set_custom_image(Some(&paintable));
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch image for '{}': {}", artist_name, e);
                    }
                }
            });
        }

        let label = gtk::Label::new(Some(artist));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.add_css_class("caption");
        label.add_css_class("dim-label");

        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        hbox.set_margin_top(8);
        hbox.set_margin_bottom(2);
        hbox.set_margin_start(0);
        hbox.set_margin_end(8);
        hbox.append(&avatar);
        hbox.append(&label);

        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&hbox));
        row.set_activatable(false);
        row.set_selectable(false);
        row
    }

    fn create_song_row(tab: &SavedTabSummary) -> gtk::ListBoxRow {
        let label = gtk::Label::new(Some(&tab.song_name));
        label.set_xalign(0.0);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_margin_top(4);
        label.set_margin_bottom(4);
        label.set_margin_start(38);
        label.set_margin_end(6);

        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&label));
        row.set_widget_name(&format!("tab:{}", tab.tab_url));
        row
    }

    fn bytes_to_texture(data: &[u8]) -> Option<gtk::gdk::Texture> {
        let loader = gtk::gdk_pixbuf::PixbufLoader::new();
        loader.write(data).ok()?;
        loader.close().ok()?;
        let pixbuf = loader.pixbuf()?;
        Some(gtk::gdk::Texture::for_pixbuf(&pixbuf))
    }
}
