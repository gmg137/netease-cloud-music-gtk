//
// tray.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::app::Action;
use crossbeam_channel::Sender;
use gtk::prelude::*;
use gtk::{Builder, Menu, MenuItem, StatusIcon};

#[derive(Clone)]
pub(crate) struct Tray {
    icon: StatusIcon,
    menu: Menu,
    exit: MenuItem,
}

impl Tray {
    pub(crate) fn new(builder: &Builder, sender: Sender<Action>) -> Self {
        let icon: StatusIcon = builder
            .get_object("status_icon")
            .expect("没找到 status_icon!");
        icon.set_property_icon_name(Some("netease-cloud-music-gtk"));
        let menu = builder
            .get_object("status_menu")
            .expect("没找到 status_menu!");
        let exit = builder
            .get_object("status_exit")
            .expect("没找到 status_exit!");
        let s = Tray { icon, menu, exit };
        Self::init(&s, sender);
        s
    }

    fn init(s: &Self, sender: Sender<Action>) {
        let menu = s.menu.downgrade();
        s.icon.connect_button_press_event(move |_, event| {
            if event.get_event_type() == gdk::EventType::ButtonPress && event.get_button() == 3 {
                menu.upgrade().unwrap().popup_easy(3, event.get_time());
            }
            true
        });

        s.exit.connect_activate(move |_| {
            sender.send(Action::QuitMain).unwrap_or(());
        });
    }
}
