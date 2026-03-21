use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use gtk4::prelude::*;
use gtk4::{self as gtk, gio, glib};
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::config::Settings;
use crate::data::cache::Cache;
use crate::data::models::{ParsedLine, TabData};
use crate::music::parser::parse_tab_content;
use crate::music::transpose::transpose_lines;
use crate::ui::chord_diagrams::ChordDiagramPanel;
use crate::ui::controls::Controls;
use crate::ui::preferences::PreferencesManager;
use crate::ui::search_results::SearchResultsView;
use crate::ui::sidebar::Sidebar;
use crate::ui::tab_view::TabView;

pub struct Window {
    pub window: adw::ApplicationWindow,
    pub sidebar: Sidebar,
    pub tab_view: TabView,
    pub search_results: SearchResultsView,
    pub controls: Controls,
    pub chord_panel: ChordDiagramPanel,
    pub preferences: PreferencesManager,
    pub content_stack: gtk::Stack,
    pub search_bar: gtk::SearchBar,
    pub search_entry: gtk::SearchEntry,
    pub title_widget: adw::WindowTitle,
    pub split_view: adw::OverlaySplitView,
    pub capo_label: gtk::Label,
    pub back_button: gtk::Button,

    pub settings: Settings,
    pub cache: Arc<Cache>,
    pub current_tab: Rc<RefCell<Option<TabData>>>,
    pub parsed_lines: Rc<RefCell<Vec<ParsedLine>>>,
    pub transpose_steps: Rc<Cell<i32>>,

    has_search_results: Rc<Cell<bool>>,
    search_query: Rc<RefCell<String>>,
    search_result_count: Rc<Cell<usize>>,
    search_scroll_pos: Rc<Cell<f64>>,

    // Infinite-scroll pagination state
    search_next_page: Rc<Cell<u32>>,
    search_is_fetching: Rc<Cell<bool>>,
    search_groups: Rc<RefCell<Vec<crate::data::models::SongGroup>>>,
}

impl Window {
    pub fn new(app: &adw::Application, cache: Arc<Cache>) -> Rc<Self> {
        let settings = Settings::new();
        let title_widget = adw::WindowTitle::new("Chords", "");

        let header_bar = adw::HeaderBar::new();
        header_bar.set_title_widget(Some(&title_widget));

        // Back button
        let back_button = gtk::Button::from_icon_name("go-previous-symbolic");
        back_button.set_tooltip_text(Some("Back to results"));
        back_button.add_css_class("flat");
        back_button.set_visible(false);
        header_bar.pack_start(&back_button);

        // Sidebar toggle button (for OverlaySplitView)
        let sidebar_button = gtk::ToggleButton::new();
        sidebar_button.set_icon_name("sidebar-show-symbolic");
        sidebar_button.set_tooltip_text(Some("Toggle sidebar"));
        sidebar_button.add_css_class("flat");
        sidebar_button.set_active(true);
        header_bar.pack_start(&sidebar_button);

        // Search
        let search_entry = gtk::SearchEntry::new();
        search_entry.set_hexpand(true);
        search_entry.set_placeholder_text(Some("Search for songs..."));

        let search_bar = gtk::SearchBar::new();
        search_bar.set_child(Some(&search_entry));
        search_bar.set_search_mode(false);
        search_bar.connect_entry(&search_entry);

        let search_button = gtk::ToggleButton::new();
        search_button.set_icon_name("system-search-symbolic");
        search_button.set_tooltip_text(Some("Search (Ctrl+F)"));
        search_button.add_css_class("flat");
        search_button
            .bind_property("active", &search_bar, "search-mode-enabled")
            .bidirectional()
            .sync_create()
            .build();

        // Controls
        let controls = Controls::new();

        header_bar.pack_end(&controls.menu_button);
        header_bar.pack_end(&search_button);
        header_bar.pack_end(&controls.scroll_button);
        header_bar.pack_end(&controls.transpose_button);
        header_bar.pack_end(&controls.save_button);

        // Preferences
        let preferences = PreferencesManager::new();

        // Tab view
        let tab_view = TabView::new();

        // Chord diagram panel
        let chord_panel = ChordDiagramPanel::new();

        // Capo pill
        let capo_label = gtk::Label::new(None);
        capo_label.set_visible(false);
        capo_label.set_halign(gtk::Align::End);
        capo_label.set_valign(gtk::Align::Start);
        capo_label.set_margin_top(8);
        capo_label.set_margin_end(8);
        capo_label.add_css_class("capo-pill");

        // Tab content area
        let tab_inner_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        tab_inner_box.append(&tab_view.scrolled_window);
        tab_inner_box.append(&controls.scroll_overlay);
        tab_inner_box.append(&chord_panel.revealer);

        let content_overlay = gtk::Overlay::new();
        content_overlay.set_child(Some(&tab_inner_box));
        content_overlay.add_overlay(&capo_label);

        // Search results
        let search_results = SearchResultsView::new();

        // Empty state — use our app icon
        let empty_status = adw::StatusPage::new();
        empty_status.set_icon_name(Some("io.github.bjesus.Chords"));
        empty_status.set_title("Chords");
        empty_status.set_description(Some(
            "Search for songs or select a saved chord from the sidebar",
        ));

        // Loading
        let loading_box = gtk::Box::new(gtk::Orientation::Vertical, 12);
        loading_box.set_valign(gtk::Align::Center);
        loading_box.set_halign(gtk::Align::Center);
        let spinner = gtk::Spinner::new();
        spinner.set_spinning(true);
        spinner.set_width_request(32);
        spinner.set_height_request(32);
        let loading_label = gtk::Label::new(Some("Loading..."));
        loading_label.add_css_class("dim-label");
        loading_box.append(&spinner);
        loading_box.append(&loading_label);

        // Content stack
        let content_stack = gtk::Stack::new();
        content_stack.set_vexpand(true);
        content_stack.set_hexpand(true);
        content_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
        content_stack.set_transition_duration(150);
        content_stack.add_named(&empty_status, Some("empty"));
        content_stack.add_named(&content_overlay, Some("tab"));
        content_stack.add_named(&search_results.scrolled_window, Some("search"));
        content_stack.add_named(&loading_box, Some("loading"));
        content_stack.set_visible_child_name("empty");

        // Content page
        let content_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content_box.append(&header_bar);
        content_box.append(&search_bar);
        content_box.append(&content_stack);

        // Sidebar
        let sidebar = Sidebar::new(&cache);

        // OverlaySplitView — supports collapsing and overlay mode
        let split_view = adw::OverlaySplitView::new();
        split_view.set_sidebar(Some(&sidebar.container));
        split_view.set_content(Some(&content_box));
        split_view.set_min_sidebar_width(200.0);
        split_view.set_max_sidebar_width(280.0);
        split_view.set_show_sidebar(true);

        // Bind sidebar toggle button to split view — always visible
        sidebar_button
            .bind_property("active", &split_view, "show-sidebar")
            .bidirectional()
            .sync_create()
            .build();

        // Window — restore saved size
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .default_width(settings.window_width())
            .default_height(settings.window_height())
            .content(&split_view)
            .build();

        if settings.is_maximized() {
            window.maximize();
        }

        // Breakpoint: collapse sidebar when window is narrow (<600sp)
        let breakpoint = adw::Breakpoint::new(
            adw::BreakpointCondition::parse("max-width: 600sp").unwrap(),
        );
        breakpoint.add_setter(&split_view, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);

        // CSS
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_data(
            r#"
            .capo-pill {
                background: alpha(@accent_color, 0.15);
                color: @accent_color;
                border-radius: 12px;
                padding: 4px 12px;
                font-weight: bold;
                font-size: 12px;
            }
            .chord-diagram {
                background: alpha(@card_bg_color, 0.8);
                border-radius: 8px;
                padding: 8px;
            }
            .dim-label {
                opacity: 0.55;
            }
            "#,
        );
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let win = Rc::new(Window {
            window,
            sidebar,
            tab_view,
            search_results,
            controls,
            chord_panel,
            preferences,
            content_stack,
            search_bar,
            search_entry,
            title_widget,
            split_view,
            capo_label,
            back_button,
            settings,
            cache,
            current_tab: Rc::new(RefCell::new(None)),
            parsed_lines: Rc::new(RefCell::new(Vec::new())),
            transpose_steps: Rc::new(Cell::new(0)),
            has_search_results: Rc::new(Cell::new(false)),
            search_query: Rc::new(RefCell::new(String::new())),
            search_result_count: Rc::new(Cell::new(0)),
            search_scroll_pos: Rc::new(Cell::new(0.0)),
            search_next_page: Rc::new(Cell::new(0)),
            search_is_fetching: Rc::new(Cell::new(false)),
            search_groups: Rc::new(RefCell::new(Vec::new())),
        });

        win.apply_settings();
        win.connect_signals();
        win.setup_actions_and_shortcuts();
        win
    }

    /// Apply saved settings to the UI on startup.
    fn apply_settings(self: &Rc<Self>) {
        let s = &self.settings;

        // Font
        self.tab_view.set_font_family(&s.font_family());
        self.tab_view.set_font_size(s.font_size());

        // Colors
        self.tab_view.set_chord_color(&s.chord_color());
        self.tab_view.set_section_color(&s.section_color());

        // Columns
        self.tab_view.set_columns(s.columns());

        // Chord diagrams
        if s.show_chord_diagrams() {
            self.chord_panel.set_visible(true);
        }
    }

    fn connect_signals(self: &Rc<Self>) {
        let win = Rc::clone(self);

        // Save window state on close
        {
            let s = self.settings.clone();
            self.window.connect_close_request(move |window| {
                let (width, height) = window.default_size();
                s.set_window_width(width);
                s.set_window_height(height);
                s.set_is_maximized(window.is_maximized());
                glib::Propagation::Proceed
            });
        }

        // Search
        {
            let w = Rc::clone(&win);
            self.search_entry.connect_activate(move |entry| {
                let query = entry.text().to_string();
                if !query.trim().is_empty() {
                    w.perform_search(query);
                }
            });
        }

        // Search result clicked
        {
            let w = Rc::clone(&win);
            self.search_results.connect_activated(move |url| {
                let vadj = w.search_results.scrolled_window.vadjustment();
                w.search_scroll_pos.set(vadj.value());
                w.load_tab(url);
            });
        }

        // Infinite scroll: load next page when user reaches the bottom
        {
            let w = Rc::clone(&win);
            self.search_results.scrolled_window.connect_edge_reached(move |_, pos| {
                if pos == gtk::PositionType::Bottom {
                    let w = Rc::clone(&w);
                    glib::spawn_future_local(async move {
                        w.load_next_search_page(true).await;
                    });
                }
            });
        }

        // Sidebar saved tab clicked
        {
            let w = Rc::clone(&win);
            self.sidebar.connect_activated(move |url| {
                w.has_search_results.set(false);
                w.back_button.set_visible(false);
                // Close the sidebar overlay when collapsed (narrow window)
                if w.split_view.is_collapsed() {
                    w.split_view.set_show_sidebar(false);
                }
                w.load_tab_from_cache(url);
            });
        }

        // Back button
        {
            let w = Rc::clone(&win);
            self.back_button.connect_clicked(move |_| {
                w.go_back_to_results();
            });
        }

        // Transpose
        {
            let w = Rc::clone(&win);
            let w2 = Rc::clone(&win);
            let w3 = Rc::clone(&win);
            self.controls.connect_transpose_up(move || {
                let c = w.transpose_steps.get();
                if c < 11 { w.transpose_steps.set(c + 1); w.apply_transpose(); }
            });
            self.controls.connect_transpose_down(move || {
                let c = w2.transpose_steps.get();
                if c > -11 { w2.transpose_steps.set(c - 1); w2.apply_transpose(); }
            });
            self.controls.connect_transpose_reset(move || {
                w3.transpose_steps.set(0);
                w3.apply_transpose();
            });
        }

        // Auto-scroll toggle
        {
            let tv = self.tab_view.clone();
            let w = self.window.clone();
            self.controls.connect_scroll_toggle(move |active| {
                if active { tv.start_autoscroll(w.upcast_ref()); }
                else { tv.stop_autoscroll(w.upcast_ref()); }
            });
        }

        // Scroll speed
        {
            let tv = self.tab_view.clone();
            let w = self.window.clone();
            self.controls.connect_scroll_speed_changed(move |ms| {
                tv.set_scroll_speed(ms as u32, w.upcast_ref());
            });
        }

        // Save
        {
            let w = Rc::clone(&win);
            self.controls.save_button.connect_clicked(move |_| {
                w.toggle_save();
            });
        }

        // --- Preferences callbacks (font, colors) → apply + save ---
        {
            let tv = self.tab_view.clone();
            let s = self.settings.clone();
            self.preferences.connect_font_family_changed(move |family| {
                tv.set_font_family(&family);
                s.set_font_family(&family);
            });
        }
        {
            let tv = self.tab_view.clone();
            let s = self.settings.clone();
            self.preferences.connect_font_size_changed(move |size| {
                tv.set_font_size(size);
                s.set_font_size(size);
            });
        }
        {
            let tv = self.tab_view.clone();
            let s = self.settings.clone();
            self.preferences.connect_chord_color_changed(move |color| {
                tv.set_chord_color(&color);
                s.set_chord_color(&color);
            });
        }
        {
            let tv = self.tab_view.clone();
            let s = self.settings.clone();
            self.preferences.connect_section_color_changed(move |color| {
                tv.set_section_color(&color);
                s.set_section_color(&color);
            });
        }
    }

    fn setup_actions_and_shortcuts(self: &Rc<Self>) {
        // ── Standard actions ─────────────────────

        // Toggle online search (Ctrl+F)
        let search_action = gio::SimpleAction::new("toggle-search", None);
        {
            let sb = self.search_bar.clone();
            let se = self.search_entry.clone();
            search_action.connect_activate(move |_, _| {
                let active = sb.is_search_mode();
                sb.set_search_mode(!active);
                if !active { se.grab_focus(); }
            });
        }
        self.window.add_action(&search_action);

        // Go back (Alt+Left)
        let back_action = gio::SimpleAction::new("go-back", None);
        {
            let w = Rc::clone(self);
            back_action.connect_activate(move |_, _| {
                if w.has_search_results.get()
                    && w.content_stack.visible_child_name().as_deref() == Some("tab")
                {
                    w.go_back_to_results();
                }
            });
        }
        self.window.add_action(&back_action);

        // Quit (Ctrl+Q)
        let quit_action = gio::SimpleAction::new("quit", None);
        {
            let window = self.window.clone();
            quit_action.connect_activate(move |_, _| { window.close(); });
        }
        self.window.add_action(&quit_action);

        // Zoom in / out (Ctrl++/-)
        {
            let tv = self.tab_view.clone();
            let s = self.settings.clone();
            let zoom_in = gio::SimpleAction::new("zoom-in", None);
            zoom_in.connect_activate(move |_, _| {
                let new_size = tv.get_font_size() + 1;
                tv.set_font_size(new_size);
                s.set_font_size(new_size.clamp(8, 32));
            });
            self.window.add_action(&zoom_in);
        }
        {
            let tv = self.tab_view.clone();
            let s = self.settings.clone();
            let zoom_out = gio::SimpleAction::new("zoom-out", None);
            zoom_out.connect_activate(move |_, _| {
                let new_size = tv.get_font_size() - 1;
                tv.set_font_size(new_size);
                s.set_font_size(new_size.clamp(8, 32));
            });
            self.window.add_action(&zoom_out);
        }

        // Filter library (Ctrl+K)
        {
            let sidebar = self.sidebar.clone();
            let split = self.split_view.clone();
            let filter_action = gio::SimpleAction::new("filter-library", None);
            filter_action.connect_activate(move |_, _| {
                split.set_show_sidebar(true);
                sidebar.show_filter();
            });
            self.window.add_action(&filter_action);
        }

        // ── Hamburger menu actions ───────────────

        // Show Chord Diagrams (stateful toggle — init from settings)
        {
            let panel = self.chord_panel.clone();
            let s = self.settings.clone();
            let initial = s.show_chord_diagrams();
            let action = gio::SimpleAction::new_stateful(
                "show-chord-diagrams",
                None,
                &initial.to_variant(),
            );
            action.connect_activate(move |action, _| {
                let current = action.state().and_then(|v| v.get::<bool>()).unwrap_or(false);
                let new_state = !current;
                action.set_state(&new_state.to_variant());
                panel.set_visible(new_state);
                s.set_show_chord_diagrams(new_state);
            });
            self.window.add_action(&action);
        }

        // Columns (stateful string radio — init from settings)
        {
            let tv = self.tab_view.clone();
            let s = self.settings.clone();
            let initial = s.columns().to_string();
            let action = gio::SimpleAction::new_stateful(
                "columns",
                Some(&String::static_variant_type()),
                &initial.to_variant(),
            );
            action.connect_activate(move |action, param| {
                if let Some(val) = param.and_then(|v| v.get::<String>()) {
                    action.set_state(&val.to_variant());
                    if let Ok(n) = val.parse::<i32>() {
                        tv.set_columns(n);
                        s.set_columns(n);
                    }
                }
            });
            self.window.add_action(&action);
        }

        // Preferences
        {
            let w = Rc::clone(self);
            let prefs_action = gio::SimpleAction::new("preferences", None);
            prefs_action.connect_activate(move |_, _| {
                w.preferences.present(&w.window, &w.settings);
            });
            self.window.add_action(&prefs_action);
        }

        // Keyboard Shortcuts
        {
            let window = self.window.clone();
            let shortcuts_action = gio::SimpleAction::new("show-shortcuts", None);
            shortcuts_action.connect_activate(move |_, _| {
                Self::show_shortcuts_window(&window);
            });
            self.window.add_action(&shortcuts_action);
        }

        // Print
        {
            let w = Rc::clone(self);
            let print_action = gio::SimpleAction::new("print", None);
            print_action.connect_activate(move |_, _| {
                w.print_current_tab();
            });
            self.window.add_action(&print_action);
        }

        // About
        {
            let window = self.window.clone();
            let cache = self.cache.clone();
            let about_action = gio::SimpleAction::new("about", None);
            about_action.connect_activate(move |_, _| {
                Self::show_about_dialog(&window, &cache);
            });
            self.window.add_action(&about_action);
        }

        // ── Space key (event controller, capture phase) ──
        let scroll_btn = self.controls.scroll_button.clone();
        let content_stack = self.content_stack.clone();
        let search_bar = self.search_bar.clone();
        let filter_bar_active = {
            let sidebar = self.sidebar.clone();
            move || sidebar.filter_entry.has_focus()
        };
        let key_controller = gtk::EventControllerKey::new();
        key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::space {
                if !search_bar.is_search_mode()
                    && !filter_bar_active()
                    && content_stack.visible_child_name().as_deref() == Some("tab")
                {
                    scroll_btn.set_active(!scroll_btn.is_active());
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(key_controller);

        // ── Keyboard shortcuts ───────────────────
        let sc = gtk::ShortcutController::new();
        sc.set_scope(gtk::ShortcutScope::Managed);

        let shortcuts = [
            ("<Ctrl>f", "win.toggle-search"),
            ("<Ctrl>k", "win.filter-library"),
            ("<Alt>Left", "win.go-back"),
            ("<Ctrl>q", "win.quit"),
            ("<Ctrl>plus", "win.zoom-in"),
            ("<Ctrl>equal", "win.zoom-in"),
            ("<Ctrl>minus", "win.zoom-out"),
        ];

        for (trigger, action) in shortcuts {
            sc.add_shortcut(gtk::Shortcut::new(
                gtk::ShortcutTrigger::parse_string(trigger),
                Some(gtk::NamedAction::new(action)),
            ));
        }

        self.window.add_controller(sc);
    }

    fn print_current_tab(self: &Rc<Self>) {
        let tab = self.current_tab.borrow();
        if tab.is_none() {
            return;
        }

        let print_op = gtk::PrintOperation::new();
        print_op.set_n_pages(1);

        let lines = self.parsed_lines.borrow().clone();
        let font_family = self.settings.font_family();
        let font_size = self.settings.font_size();

        print_op.connect_draw_page(move |_, ctx, _page_nr| {
            let cr = ctx.cairo_context();
            cr.set_source_rgb(0.0, 0.0, 0.0);
            cr.set_font_size(font_size as f64);

            let layout = ctx.create_pango_layout();
            let font_desc = gtk::pango::FontDescription::from_string(
                &format!("{} {}", font_family, font_size),
            );
            layout.set_font_description(Some(&font_desc));

            let text: String = lines
                .iter()
                .map(|l| l.content.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            layout.set_text(&text);

            pangocairo::functions::show_layout(&cr, &layout);
        });

        let _ = print_op.run(
            gtk::PrintOperationAction::PrintDialog,
            Some(&self.window),
        );
    }

    fn show_about_dialog(parent: &adw::ApplicationWindow, _cache: &Arc<Cache>) {
        let db_path = Cache::db_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        let about = adw::AboutDialog::new();
        about.set_application_name("Chords");
        about.set_version("0.2.2");
        about.set_comments("A guitar chords viewer for GNOME");
        about.set_license_type(gtk::License::Gpl30);
        about.set_application_icon("io.github.bjesus.Chords");
        about.set_website("https://github.com/bjesus/chords");
        about.set_developers(&["Chords Contributors"]);
        about.set_copyright("\u{00a9} 2026 Chords Contributors");
        about.set_debug_info(&format!("Database: {}", db_path));
        about.set_debug_info_filename("chords-debug-info.txt");
        about.present(Some(parent));
    }

    fn show_shortcuts_window(parent: &adw::ApplicationWindow) {
        let builder = gtk::Builder::from_string(
            r#"
            <interface>
              <object class="GtkShortcutsWindow" id="shortcuts">
                <property name="modal">true</property>
                <child>
                  <object class="GtkShortcutsSection">
                    <property name="visible">true</property>
                    <property name="section-name">shortcuts</property>
                    <child>
                      <object class="GtkShortcutsGroup">
                        <property name="visible">true</property>
                        <property name="title">General</property>
                        <child>
                          <object class="GtkShortcutsShortcut">
                            <property name="visible">true</property>
                            <property name="accelerator">&lt;Ctrl&gt;f</property>
                            <property name="title">Search Online</property>
                          </object>
                        </child>
                        <child>
                          <object class="GtkShortcutsShortcut">
                            <property name="visible">true</property>
                            <property name="accelerator">&lt;Ctrl&gt;k</property>
                            <property name="title">Filter Library</property>
                          </object>
                        </child>
                        <child>
                          <object class="GtkShortcutsShortcut">
                            <property name="visible">true</property>
                            <property name="accelerator">&lt;Ctrl&gt;q</property>
                            <property name="title">Quit</property>
                          </object>
                        </child>
                      </object>
                    </child>
                    <child>
                      <object class="GtkShortcutsGroup">
                        <property name="visible">true</property>
                        <property name="title">Viewing</property>
                        <child>
                          <object class="GtkShortcutsShortcut">
                            <property name="visible">true</property>
                            <property name="accelerator">space</property>
                            <property name="title">Toggle Auto-scroll</property>
                          </object>
                        </child>
                        <child>
                          <object class="GtkShortcutsShortcut">
                            <property name="visible">true</property>
                            <property name="accelerator">&lt;Ctrl&gt;plus</property>
                            <property name="title">Increase Font Size</property>
                          </object>
                        </child>
                        <child>
                          <object class="GtkShortcutsShortcut">
                            <property name="visible">true</property>
                            <property name="accelerator">&lt;Ctrl&gt;minus</property>
                            <property name="title">Decrease Font Size</property>
                          </object>
                        </child>
                        <child>
                          <object class="GtkShortcutsShortcut">
                            <property name="visible">true</property>
                            <property name="accelerator">&lt;Alt&gt;Left</property>
                            <property name="title">Back to Search Results</property>
                          </object>
                        </child>
                      </object>
                    </child>
                  </object>
                </child>
              </object>
            </interface>
            "#,
        );

        let shortcuts_window: gtk::ShortcutsWindow = builder.object("shortcuts").unwrap();
        shortcuts_window.set_transient_for(Some(parent));
        shortcuts_window.present();
    }

    pub fn perform_search(self: &Rc<Self>, query: String) {
        // Reset all pagination state for a fresh search
        self.search_results.clear();
        self.search_results.set_loading_more(false);
        self.content_stack.set_visible_child_name("loading");
        self.set_controls_sensitive(false);
        self.has_search_results.set(false);
        self.back_button.set_visible(false);
        self.search_next_page.set(1);
        self.search_is_fetching.set(false);
        *self.search_groups.borrow_mut() = Vec::new();
        *self.search_query.borrow_mut() = query.clone();
        self.search_scroll_pos.set(0.0);

        // Fetch the first page immediately (no delay)
        let win = Rc::clone(self);
        glib::spawn_future_local(async move {
            win.load_next_search_page(false).await;
        });
    }

    /// Fetch and merge the next page of results.
    /// `delay`: if true, wait 3 seconds first (courtesy delay for scroll-triggered loads).
    async fn load_next_search_page(self: &Rc<Self>, delay: bool) {
        // Guards
        if self.search_is_fetching.get() { return; }
        let page = self.search_next_page.get();
        if page == 0 { return; } // exhausted

        self.search_is_fetching.set(true);
        self.search_results.set_loading_more(true);

        if delay {
            glib::timeout_future(std::time::Duration::from_secs(3)).await;
            // Re-check guards after sleep (user may have started a new search)
            if self.search_next_page.get() != page {
                self.search_is_fetching.set(false);
                return;
            }
        }

        let query = self.search_query.borrow().clone();

        match crate::data::api::search(&query, page).await {
            Ok(response) => {
                if response.results.is_empty() {
                    // No more pages
                    self.search_next_page.set(0);
                    self.search_results.set_loading_more(false);
                    self.search_is_fetching.set(false);
                    // Update subtitle to remove the trailing "…"
                    let count = self.search_result_count.get();
                    self.title_widget.set_subtitle(&format!(
                        "{} song{} for \"{}\"",
                        count,
                        if count == 1 { "" } else { "s" },
                        query
                    ));
                    return;
                }

                // Merge into accumulated groups
                {
                    let mut groups = self.search_groups.borrow_mut();
                    crate::data::models::SongGroup::merge(&mut groups, response.results);
                }
                let snapshot = self.search_groups.borrow().clone();
                let count = snapshot.len();

                if page == 1 {
                    self.search_results.set_groups(&snapshot);
                    self.content_stack.set_visible_child_name("search");
                    self.has_search_results.set(true);
                } else {
                    self.search_results.update_groups(&snapshot);
                }

                self.title_widget.set_title("Search Results");
                self.title_widget.set_subtitle(&format!(
                    "{} song{} for \"{}\"…",
                    count,
                    if count == 1 { "" } else { "s" },
                    query
                ));
                self.search_result_count.set(count);
                self.search_next_page.set(page + 1);
                self.search_results.set_loading_more(false);
                self.search_is_fetching.set(false);

                // If results don't fill the viewport, load another page immediately
                self.try_load_more_if_needed();
            }
            Err(e) => {
                eprintln!("Search page {} error: {}", page, e);
                if page == 1 {
                    self.content_stack.set_visible_child_name("empty");
                    self.title_widget.set_title("Chords");
                    self.title_widget.set_subtitle(&format!("Search failed: {}", e));
                }
                self.search_results.set_loading_more(false);
                self.search_is_fetching.set(false);
            }
        }
    }

    /// If the current search results don't fill the viewport, load the next page
    /// immediately (no delay) so the list is scrollable before requiring user action.
    fn try_load_more_if_needed(self: &Rc<Self>) {
        if self.search_next_page.get() == 0 { return; }

        let win = Rc::clone(self);
        glib::idle_add_local_once(move || {
            let vadj = win.search_results.scrolled_window.vadjustment();
            // If content height ≤ viewport height, there's nothing to scroll — load more
            if vadj.upper() <= vadj.page_size() + 1.0
                && win.search_next_page.get() > 0
                && !win.search_is_fetching.get()
            {
                glib::spawn_future_local(async move {
                    win.load_next_search_page(false).await;
                });
            }
        });
    }

    fn go_back_to_results(self: &Rc<Self>) {
        if !self.has_search_results.get() { return; }

        self.content_stack.set_visible_child_name("search");
        let query = self.search_query.borrow().clone();
        let count = self.search_result_count.get();
        self.title_widget.set_title("Search Results");
        self.title_widget.set_subtitle(&format!(
            "{} song{} for \"{}\"",
            count,
            if count == 1 { "" } else { "s" },
            query
        ));
        self.back_button.set_visible(false);
        self.set_controls_sensitive(false);

        let scroll_pos = self.search_scroll_pos.get();
        let scrolled = self.search_results.scrolled_window.clone();
        glib::idle_add_local_once(move || {
            scrolled.vadjustment().set_value(scroll_pos);
        });
    }

    pub fn load_tab(self: &Rc<Self>, tab_url: String) {
        if let Some(tab) = self.cache.get_saved_tab(&tab_url).ok().flatten() {
            self.display_tab(tab);
            return;
        }

        let win = Rc::clone(self);
        win.content_stack.set_visible_child_name("loading");

        glib::spawn_future_local(async move {
            match crate::data::api::fetch_tab(&tab_url).await {
                Ok(tab) => win.display_tab(tab),
                Err(e) => {
                    eprintln!("Tab fetch error: {}", e);
                    win.content_stack.set_visible_child_name("empty");
                    win.title_widget.set_subtitle(&format!("Failed to load: {}", e));
                }
            }
        });
    }

    pub fn load_tab_from_cache(self: &Rc<Self>, tab_url: String) {
        match self.cache.get_saved_tab(&tab_url) {
            Ok(Some(tab)) => self.display_tab(tab),
            _ => self.load_tab(tab_url),
        }
    }

    fn display_tab(self: &Rc<Self>, tab: TabData) {
        self.transpose_steps.set(0);
        self.controls.update_transpose_label(0);

        self.title_widget
            .set_title(&format!("{} \u{2013} {}", tab.artist_name, tab.song_name));

        let mut parts = Vec::new();
        if tab.version > 1 {
            parts.push(format!("ver {}", tab.version));
        }
        if let Some(ref tuning) = tab.tuning {
            if tuning.name != "Standard" {
                parts.push(tuning.name.clone());
            }
        }
        self.title_widget.set_subtitle(&parts.join(" · "));

        let lines = parse_tab_content(&tab.raw_content);
        self.tab_view.render_lines(&lines);

        match &tab.capo {
            Some(capo) if capo != "0" => {
                self.capo_label.set_text(&format!("Capo {}", capo));
                self.capo_label.set_visible(true);
            }
            _ => self.capo_label.set_visible(false),
        }

        // Extract unique chord names from parsed lines for diagram lookup
        let unique_chords = Self::extract_unique_chords(&lines);
        if !unique_chords.is_empty() {
            self.chord_panel.set_chord_names(&unique_chords);
        } else {
            self.chord_panel.clear();
        }

        self.controls.update_save_state(self.cache.is_saved(&tab.tab_url));

        *self.parsed_lines.borrow_mut() = lines;
        *self.current_tab.borrow_mut() = Some(tab);

        self.content_stack.set_visible_child_name("tab");
        self.search_bar.set_search_mode(false);
        self.set_controls_sensitive(true);

        self.back_button.set_visible(self.has_search_results.get());
    }

    fn apply_transpose(self: &Rc<Self>) {
        let steps = self.transpose_steps.get();
        self.controls.update_transpose_label(steps);
        self.chord_panel.set_transpose(steps);

        let original_lines = {
            let tab = self.current_tab.borrow();
            if let Some(ref tab) = *tab {
                parse_tab_content(&tab.raw_content)
            } else {
                return;
            }
        };

        let transposed = transpose_lines(&original_lines, steps);
        self.tab_view.render_lines(&transposed);
        *self.parsed_lines.borrow_mut() = transposed;
    }

    fn toggle_save(self: &Rc<Self>) {
        let tab = self.current_tab.borrow().clone();
        if let Some(tab) = tab {
            if self.cache.is_saved(&tab.tab_url) {
                let _ = self.cache.remove_tab(&tab.tab_url);
                self.controls.update_save_state(false);
            } else {
                let _ = self.cache.save_tab(&tab);
                self.controls.update_save_state(true);
            }
            self.sidebar.refresh(&self.cache);
        }
    }

    /// Extract unique chord names from parsed lines, preserving order of first appearance.
    fn extract_unique_chords(lines: &[ParsedLine]) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for line in lines {
            for chord in &line.chords {
                let name = chord.display();
                if seen.insert(name.clone()) {
                    result.push(name);
                }
            }
        }
        result
    }

    fn set_controls_sensitive(&self, sensitive: bool) {
        self.controls.transpose_button.set_sensitive(sensitive);
        self.controls.scroll_button.set_sensitive(sensitive);
        self.controls.save_button.set_sensitive(sensitive);
    }
}
