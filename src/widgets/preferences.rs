//
// preferences.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::app::Action;
use crate::utils::Configs;
use crossbeam_channel::Sender;
use gtk::prelude::*;
use gtk::{Builder, Dialog, Switch};

#[derive(Clone)]
pub(crate) struct Preferences {
    pub(crate) dialog: Dialog,
    tray: Switch,
    lyrics: Switch,
}

impl Preferences {
    pub(crate) fn new(builder: &Builder, sender: Sender<Action>, configs: &Configs) -> Self {
        let dialog: Dialog = builder
            .get_object("preferences_dialog")
            .expect("没找到 preferences_dialog");
        let tray: Switch = builder
            .get_object("config_tray_switch")
            .expect("没找到 config_tray_switch");
        let lyrics: Switch = builder
            .get_object("config_lyrics_switch")
            .expect("没找到 config_lyrics_switch");
        tray.set_state(configs.tray);
        lyrics.set_state(configs.lyrics);

        let sender_clone = sender.clone();
        tray.connect_state_set(move |_, state| {
            sender_clone
                .send(Action::ConfigsSetTray(state))
                .unwrap_or(());
            Inhibit(false)
        });

        let sender_clone = sender.clone();
        lyrics.connect_state_set(move |_, state| {
            sender_clone
                .send(Action::ConfigsSetLyrics(state))
                .unwrap_or(());
            Inhibit(false)
        });

        Preferences {
            dialog,
            tray,
            lyrics,
        }
    }
}
