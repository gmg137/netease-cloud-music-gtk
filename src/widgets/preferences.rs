//
// preferences.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::{
    app::Action,
    utils::{ClearCached, Configs},
};
use crossbeam_channel::Sender;
use gtk::{prelude::*, Builder, ComboBoxText, Dialog, Switch};

#[derive(Clone)]
pub(crate) struct Preferences {
    pub(crate) dialog: Dialog,
    tray: Switch,
    lyrics: Switch,
    clear: ComboBoxText,
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
        let clear: ComboBoxText = builder.get_object("auto_clear_cache").expect("没找到 auto_clear_cache");
        tray.set_state(configs.tray);
        lyrics.set_state(configs.lyrics);
        match configs.clear {
            ClearCached::NONE => {
                clear.set_active_id(Some("0"));
            }
            ClearCached::MONTH(_) => {
                clear.set_active_id(Some("1"));
            }
            ClearCached::WEEK(_) => {
                clear.set_active_id(Some("2"));
            }
            ClearCached::DAY(_) => {
                clear.set_active_id(Some("3"));
            }
        };

        let sender_clone = sender.clone();
        tray.connect_state_set(move |_, state| {
            sender_clone.send(Action::ConfigsSetTray(state)).unwrap_or(());
            Inhibit(false)
        });

        let sender_clone = sender.clone();
        lyrics.connect_state_set(move |_, state| {
            sender_clone.send(Action::ConfigsSetLyrics(state)).unwrap_or(());
            Inhibit(false)
        });

        let sender_clone = sender.clone();
        clear.connect_changed(move |s| {
            if let Some(id) = s.get_active_id() {
                sender_clone
                    .send(Action::ConfigsSetClear(id.parse::<u8>().unwrap_or(0)))
                    .unwrap_or(());
            }
        });

        Preferences {
            dialog,
            tray,
            lyrics,
            clear,
        }
    }
}
