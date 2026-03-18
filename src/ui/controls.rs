use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{self as gtk};

/// Header bar controls — transpose popover, auto-scroll toggle, save button,
/// and a standard gio::Menu-driven hamburger menu.
#[derive(Clone)]
pub struct Controls {
    // Transpose (single button with popover)
    pub transpose_button: gtk::MenuButton,
    transpose_label: gtk::Label,

    // Auto-scroll (single toggle button, shows overlay when active)
    pub scroll_button: gtk::ToggleButton,
    pub scroll_overlay: gtk::Revealer,

    // Hamburger menu (gio::Menu driven)
    pub menu_button: gtk::MenuButton,

    // Save (toggle button — active = saved)
    pub save_button: gtk::ToggleButton,

    // Callbacks (only for transpose and scroll — menu uses gio actions)
    transpose_up_cbs: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
    transpose_down_cbs: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
    transpose_reset_cbs: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
    scroll_toggle_cbs: Rc<RefCell<Vec<Box<dyn Fn(bool)>>>>,
    scroll_speed_cbs: Rc<RefCell<Vec<Box<dyn Fn(f64)>>>>,
}

impl Controls {
    pub fn new() -> Self {
        // ============================
        // Transpose popover
        // ============================
        let transpose_down_btn = gtk::Button::from_icon_name("go-down-symbolic");
        transpose_down_btn.add_css_class("flat");
        transpose_down_btn.set_tooltip_text(Some("Transpose down"));

        let transpose_label = gtk::Label::new(Some("0"));
        transpose_label.set_width_chars(3);
        transpose_label.set_tooltip_text(Some("Click to reset"));

        let transpose_up_btn = gtk::Button::from_icon_name("go-up-symbolic");
        transpose_up_btn.add_css_class("flat");
        transpose_up_btn.set_tooltip_text(Some("Transpose up"));

        let transpose_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        transpose_row.add_css_class("linked");
        transpose_row.set_halign(gtk::Align::Center);
        transpose_row.append(&transpose_down_btn);
        transpose_row.append(&transpose_label);
        transpose_row.append(&transpose_up_btn);

        let transpose_popover_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        transpose_popover_box.set_margin_top(8);
        transpose_popover_box.set_margin_bottom(8);
        transpose_popover_box.set_margin_start(8);
        transpose_popover_box.set_margin_end(8);
        let transpose_title = gtk::Label::new(Some("Transpose"));
        transpose_title.add_css_class("heading");
        transpose_popover_box.append(&transpose_title);
        transpose_popover_box.append(&transpose_row);

        let transpose_popover = gtk::Popover::new();
        transpose_popover.set_child(Some(&transpose_popover_box));

        let transpose_button = gtk::MenuButton::new();
        transpose_button.set_icon_name("media-playlist-shuffle-symbolic");
        transpose_button.set_popover(Some(&transpose_popover));
        transpose_button.set_tooltip_text(Some("Transpose"));
        transpose_button.add_css_class("flat");

        // ============================
        // Auto-scroll
        // ============================
        let scroll_button = gtk::ToggleButton::new();
        scroll_button.set_icon_name("media-playback-start-symbolic");
        scroll_button.set_tooltip_text(Some("Auto-scroll (Space)"));
        scroll_button.add_css_class("flat");

        let scroll_speed_scale = gtk::Scale::with_range(
            gtk::Orientation::Horizontal, 10.0, 1000.0, 50.0,
        );
        scroll_speed_scale.set_value(500.0);
        scroll_speed_scale.set_inverted(true);
        scroll_speed_scale.set_hexpand(true);

        let scroll_pause_btn = gtk::Button::from_icon_name("media-playback-pause-symbolic");
        scroll_pause_btn.add_css_class("flat");
        scroll_pause_btn.set_tooltip_text(Some("Stop scrolling"));

        let scroll_overlay_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        scroll_overlay_box.set_margin_start(16);
        scroll_overlay_box.set_margin_end(16);
        scroll_overlay_box.set_margin_top(4);
        scroll_overlay_box.set_margin_bottom(4);
        let slow_label = gtk::Label::new(Some("Slow"));
        slow_label.add_css_class("dim-label");
        slow_label.add_css_class("caption");
        let fast_label = gtk::Label::new(Some("Fast"));
        fast_label.add_css_class("dim-label");
        fast_label.add_css_class("caption");
        scroll_overlay_box.append(&scroll_pause_btn);
        scroll_overlay_box.append(&slow_label);
        scroll_overlay_box.append(&scroll_speed_scale);
        scroll_overlay_box.append(&fast_label);

        let scroll_overlay = gtk::Revealer::new();
        scroll_overlay.set_child(Some(&scroll_overlay_box));
        scroll_overlay.set_reveal_child(false);
        scroll_overlay.set_transition_type(gtk::RevealerTransitionType::SlideUp);
        scroll_overlay.set_transition_duration(200);

        {
            let btn = scroll_button.clone();
            scroll_pause_btn.connect_clicked(move |_| { btn.set_active(false); });
        }

        // ============================
        // Hamburger menu — standard gio::Menu
        // ============================
        let menu = gtk::gio::Menu::new();

        // Section 1: View toggles + Print
        let view_section = gtk::gio::Menu::new();
        view_section.append(Some("Show Chord Diagrams"), Some("win.show-chord-diagrams"));
        view_section.append(Some("Print\u{2026}"), Some("win.print"));
        menu.append_section(None, &view_section);

        // Section 2: Columns (radio)
        let columns_section = gtk::gio::Menu::new();
        columns_section.append(Some("1 Column"), Some("win.columns::1"));
        columns_section.append(Some("2 Columns"), Some("win.columns::2"));
        columns_section.append(Some("3 Columns"), Some("win.columns::3"));
        columns_section.append(Some("4 Columns"), Some("win.columns::4"));
        menu.append_section(Some("Columns"), &columns_section);

        // Section 3: Standard items
        let standard_section = gtk::gio::Menu::new();
        standard_section.append(Some("Preferences"), Some("win.preferences"));
        standard_section.append(Some("Keyboard Shortcuts"), Some("win.show-shortcuts"));
        standard_section.append(Some("About Chords"), Some("win.about"));
        menu.append_section(None, &standard_section);

        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_menu_model(Some(&menu));
        menu_button.set_tooltip_text(Some("Main Menu"));
        menu_button.add_css_class("flat");
        menu_button.set_primary(true);

        // ============================
        // Save button
        // ============================
        let save_button = gtk::ToggleButton::new();
        save_button.set_icon_name("user-bookmarks-symbolic");
        save_button.set_tooltip_text(Some("Save to library"));
        save_button.add_css_class("flat");

        // ============================
        // Build struct
        // ============================
        let controls = Controls {
            transpose_button,
            transpose_label: transpose_label.clone(),
            scroll_button: scroll_button.clone(),
            scroll_overlay,
            menu_button,
            save_button,
            transpose_up_cbs: Rc::new(RefCell::new(Vec::new())),
            transpose_down_cbs: Rc::new(RefCell::new(Vec::new())),
            transpose_reset_cbs: Rc::new(RefCell::new(Vec::new())),
            scroll_toggle_cbs: Rc::new(RefCell::new(Vec::new())),
            scroll_speed_cbs: Rc::new(RefCell::new(Vec::new())),
        };

        // Wire signals
        {
            let cbs = Rc::clone(&controls.transpose_up_cbs);
            transpose_up_btn.connect_clicked(move |_| {
                for cb in cbs.borrow().iter() { cb(); }
            });
        }
        {
            let cbs = Rc::clone(&controls.transpose_down_cbs);
            transpose_down_btn.connect_clicked(move |_| {
                for cb in cbs.borrow().iter() { cb(); }
            });
        }
        {
            let cbs = Rc::clone(&controls.transpose_reset_cbs);
            let gesture = gtk::GestureClick::new();
            gesture.connect_released(move |_, _, _, _| {
                for cb in cbs.borrow().iter() { cb(); }
            });
            transpose_label.add_controller(gesture);
        }
        {
            let cbs = Rc::clone(&controls.scroll_toggle_cbs);
            let overlay = controls.scroll_overlay.clone();
            let in_handler = Rc::new(Cell::new(false));
            scroll_button.connect_toggled(move |toggle| {
                if in_handler.get() { return; }
                in_handler.set(true);
                let active = toggle.is_active();
                overlay.set_reveal_child(active);
                for cb in cbs.borrow().iter() { cb(active); }
                in_handler.set(false);
            });
        }
        {
            let cbs = Rc::clone(&controls.scroll_speed_cbs);
            scroll_speed_scale.connect_value_changed(move |scale| {
                let val = scale.value();
                for cb in cbs.borrow().iter() { cb(val); }
            });
        }

        controls
    }

    // Callback connectors (only for transpose + scroll)
    pub fn connect_transpose_up<F: Fn() + 'static>(&self, f: F) {
        self.transpose_up_cbs.borrow_mut().push(Box::new(f));
    }
    pub fn connect_transpose_down<F: Fn() + 'static>(&self, f: F) {
        self.transpose_down_cbs.borrow_mut().push(Box::new(f));
    }
    pub fn connect_transpose_reset<F: Fn() + 'static>(&self, f: F) {
        self.transpose_reset_cbs.borrow_mut().push(Box::new(f));
    }
    pub fn connect_scroll_toggle<F: Fn(bool) + 'static>(&self, f: F) {
        self.scroll_toggle_cbs.borrow_mut().push(Box::new(f));
    }
    pub fn connect_scroll_speed_changed<F: Fn(f64) + 'static>(&self, f: F) {
        self.scroll_speed_cbs.borrow_mut().push(Box::new(f));
    }

    pub fn update_transpose_label(&self, steps: i32) {
        let text = match steps.cmp(&0) {
            std::cmp::Ordering::Greater => format!("+{}", steps),
            std::cmp::Ordering::Less => steps.to_string(),
            std::cmp::Ordering::Equal => "0".to_string(),
        };
        self.transpose_label.set_text(&text);
    }

    pub fn update_save_state(&self, is_saved: bool) {
        self.save_button.set_active(is_saved);
        self.save_button.set_tooltip_text(Some(if is_saved {
            "Remove from library"
        } else {
            "Save to library"
        }));
    }
}
