mod config;
mod data;
mod music;
mod ui;

use std::sync::Arc;

use libadwaita as adw;
use libadwaita::prelude::*;

use data::cache::Cache;
use ui::window::Window;

fn main() {
    // Point GSettings at our local compiled schemas (for cargo run without install)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let schema_dir = format!("{}/data", manifest_dir);
    std::env::set_var("GSETTINGS_SCHEMA_DIR", &schema_dir);

    // Initialize the async runtime for reqwest
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    let _guard = rt.enter();

    let app = adw::Application::builder()
        .application_id("de.chords.Chords")
        .build();

    app.connect_activate(move |app| {
        // Register our icon directory so GTK can find de.chords.Chords icon
        let icon_dir = format!("{}/data/icons", manifest_dir);
        gtk4::IconTheme::for_display(&gtk4::gdk::Display::default().unwrap())
            .add_search_path(&icon_dir);

        let cache = match Cache::open() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Failed to open cache: {}", e);
                eprintln!("Using in-memory database as fallback");
                Arc::new(Cache::open().expect("Failed to create even in-memory cache"))
            }
        };

        let win = Window::new(app, cache);
        win.window.present();
    });

    app.run();
}
