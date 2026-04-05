#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

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
    // On Windows, set up GTK data paths relative to the executable BEFORE GTK
    // initialises, so themes, schemas, and pixbuf loaders are discovered.
    #[cfg(target_os = "windows")]
    let exe_dir = setup_windows_env();
    #[cfg(not(target_os = "windows"))]
    let exe_dir: Option<std::path::PathBuf> = None;

    // Point GSettings at our local compiled schemas (for cargo run without install)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    // On Windows the schema dir is set by setup_windows_env(); on Linux set it
    // to the build-time source location so `cargo run` works without install.
    #[cfg(not(target_os = "windows"))]
    {
        let schema_dir = format!("{}/data", manifest_dir);
        std::env::set_var("GSETTINGS_SCHEMA_DIR", &schema_dir);
    }

    // Initialize the async runtime for reqwest
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    let _guard = rt.enter();

    let app = adw::Application::builder()
        .application_id("io.github.bjesus.Chords")
        .build();

    app.connect_activate(move |app| {
        // Register icon search paths
        if let Some(display) = gtk4::gdk::Display::default() {
            let theme = gtk4::IconTheme::for_display(&display);

            // Dev build: source tree icons
            #[cfg(not(target_os = "windows"))]
            theme.add_search_path(&format!("{}/data/icons", manifest_dir));

            // Windows bundle: icons next to the exe
            #[cfg(target_os = "windows")]
            if let Some(ref dir) = exe_dir {
                theme.add_search_path(dir.join("share").join("icons"));
            }
        }

        let cache = match Cache::open() {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Failed to open cache: {}", e);
                Arc::new(Cache::open().expect("Failed to create even in-memory cache"))
            }
        };

        let win = Window::new(app, cache);
        win.window.present();
    });

    app.run();
}

/// Set GTK-related environment variables to point at bundled data next to the
/// executable. Must be called BEFORE GTK is initialised (before `app.run()`).
/// Returns the directory containing `chords.exe`.
#[cfg(target_os = "windows")]
fn setup_windows_env() -> Option<std::path::PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))?;

    let share_dir = exe_dir.join("share");
    let lib_dir   = exe_dir.join("lib");

    // Force Cairo renderer — GPU shader compilation fails on many Windows drivers
    std::env::set_var("GSK_RENDERER", "cairo");

    // GTK / GLib data root (themes, icons, locale …)
    std::env::set_var("XDG_DATA_DIRS", &share_dir);

    // GSettings schemas
    std::env::set_var(
        "GSETTINGS_SCHEMA_DIR",
        share_dir.join("glib-2.0").join("schemas"),
    );

    // GDK pixbuf loaders cache (relative paths, works on any machine)
    let loaders = lib_dir
        .join("gdk-pixbuf-2.0")
        .join("2.10.0")
        .join("loaders.cache");
    if loaders.exists() {
        std::env::set_var("GDK_PIXBUF_MODULE_FILE", &loaders);
    }

    // fontconfig: use bundled fonts.conf that points at Windows system fonts
    let fonts_conf = share_dir.join("fonts").join("fonts.conf");
    if fonts_conf.exists() {
        std::env::set_var("FONTCONFIG_FILE", &fonts_conf);
    }

    Some(exe_dir)
}
