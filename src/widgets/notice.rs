//
// notice.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use glib;
use gtk;
use gtk::prelude::*;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum State {
    Shown,
    Hidden,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum SpinnerState {
    Active,
    Stopped,
}

#[derive(Debug, Clone)]
pub(crate) struct InAppNotification {
    revealer: gtk::Revealer,
    text: gtk::Label,
}

impl Default for InAppNotification {
    fn default() -> Self {
        let glade_src = include_str!("../../ui/inapp_notif.ui");
        let builder = gtk::Builder::new_from_string(glade_src);

        let revealer: gtk::Revealer = builder.get_object("revealer").unwrap();
        let text: gtk::Label = builder.get_object("text").unwrap();

        InAppNotification { revealer, text }
    }
}

/// Timer should be in milliseconds
impl InAppNotification {
    pub(crate) fn new<F>(text: &str, timer: u32, mut callback: F) -> Self
    where
        F: FnMut(gtk::Revealer) -> glib::Continue + 'static,
    {
        let notif = InAppNotification::default();
        let message = format!(r#"<span size="medium">{}</span>"#, text);
        notif.text.set_markup(&message);

        let revealer_weak = notif.revealer.downgrade();
        let mut time = 0;
        timeout_add(250, move || {
            if time < timer {
                time += 250;
                return glib::Continue(true);
            };

            let revealer = match revealer_weak.upgrade() {
                Some(r) => r,
                None => return glib::Continue(false),
            };

            callback(revealer)
        });

        notif
    }

    // This is a separate method cause in order to get a nice animation
    // the revealer should be attached to something that displays it.
    // Previously we where doing it in the constructor, which had the result
    // of the animation being skipped cause there was no parent widget to display it.
    pub(crate) fn show(&self, overlay: &gtk::Overlay) {
        overlay.add_overlay(&self.revealer);
        // We need to display the notification after the widget is added to the overlay
        // so there will be a nice animation.
        self.revealer.set_reveal_child(true);
    }

    pub(crate) fn destroy(self) {
        self.revealer.destroy();
    }
}
