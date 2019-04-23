//
// mod.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//

pub(crate) mod header;
pub(crate) mod notice;
pub(crate) mod player;
pub(crate) mod preferences;
pub(crate) mod tray;
use gtk::prelude::*;
use notice::*;

pub(crate) fn mark_all_notif(msg: String) -> InAppNotification {
    let callback = move |revealer: gtk::Revealer| {
        revealer.set_reveal_child(false);
        glib::Continue(false)
    };

    InAppNotification::new(&msg, 5000, callback)
}
