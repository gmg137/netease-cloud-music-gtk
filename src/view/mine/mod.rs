//
// mine.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::{
    app::Action,
    data::MusicData,
    musicapi::model::{SongInfo, SongList},
    utils::*,
};
use async_std::sync::{Arc, Mutex};
use glib::Sender;
use gtk::{prelude::*, Builder, Label, ListBox, ListBoxRow};
mod fm;
mod list;

#[derive(Clone)]
pub(crate) struct Mine {
    sidebar: ListBox,
    pub fmview: fm::FmView,
    pub listview: list::ListView,
    sender: Sender<Action>,
}

impl Mine {
    pub(crate) fn new(
        mine_login_builder: &Builder,
        mine_login_fm_builder: &Builder,
        mine_login_list_builder: &Builder,
        sender: Sender<Action>,
        music_data: Arc<Mutex<MusicData>>,
    ) -> Self {
        let sidebar: ListBox = mine_login_builder
            .object("mine_listbox")
            .expect("无法获取 mine_listbox .");
        let fmview = fm::FmView::new(&mine_login_fm_builder, sender.clone());
        let listview = list::ListView::new(&mine_login_list_builder, sender.clone(), music_data);

        let s = Mine {
            sidebar,
            fmview,
            listview,
            sender,
        };
        Self::init(&s);
        s
    }

    fn init(s: &Self) {
        let sender = s.sender.clone();
        let listbox = s.sidebar.downgrade();
        let popmenu = s.listview.lowview.popmenu.downgrade();
        s.listview.lowview.tree.connect_button_press_event(move |tree, event| {
            if event.event_type() == gdk::EventType::ButtonPress && event.button() == 3 {
                if let Some(row) = listbox.upgrade().unwrap().selected_row() {
                    if row.index() == 3 {
                        popmenu.upgrade().unwrap().popup_easy(3, event.time());
                    }
                }
            }
            if event.event_type() == gdk::EventType::DoubleButtonPress {
                if let Some((model, iter)) = tree.selection().selected() {
                    let id = model.value(&iter, 0).get::<u64>().unwrap_or(0);
                    let name = model.value(&iter, 1).get::<String>().unwrap_or_else(|_| "".to_owned());
                    let duration = model.value(&iter, 2).get::<String>().unwrap_or_else(|_| "".to_owned());
                    let singer = model.value(&iter, 3).get::<String>().unwrap_or_else(|_| "".to_owned());
                    let album = model.value(&iter, 4).get::<String>().unwrap_or_else(|_| "".to_owned());
                    let pic_url = model.value(&iter, 5).get::<String>().unwrap_or_else(|_| "".to_owned());
                    sender
                        .send(Action::PlayerInit(
                            SongInfo {
                                id,
                                name,
                                duration,
                                singer,
                                album,
                                pic_url,
                                song_url: String::new(),
                            },
                            PlayerTypes::Song,
                        ))
                        .unwrap_or(());
                }
            }
            Inhibit(false)
        });

        let sender = s.sender.clone();
        s.sidebar.connect_row_selected(move |_, row| {
            if let Some(row) = row.as_ref() {
                sender.send(Action::RefreshMineViewInit(row.index())).unwrap_or(());
            }
        });
    }

    pub(crate) fn update_sidebar(&self, song_list: Vec<SongList>) {
        if let Some(one_row) = self.sidebar.row_at_index(0) {
            self.sidebar.select_row(Some(&one_row));
            self.sidebar.children()[3..].iter().for_each(|w| {
                self.sidebar.remove(w);
            });
        }
        let row = self.sidebar.row_at_index(4);
        if row.is_none() {
            song_list.iter().for_each(|sl| {
                let label = Label::new(Some(&sl.name[..]));
                label.set_halign(gtk::Align::Start);
                label.set_valign(gtk::Align::Fill);
                label.set_margin_start(18);
                label.set_ellipsize(pango::EllipsizeMode::End);
                label.set_max_width_chars(16);
                let row = ListBoxRow::new();
                row.set_height_request(58);
                row.add(&label);
                self.sidebar.insert(&row, -1);
            });
        }
        self.sidebar.show_all();
    }

    pub(crate) fn selected_row_id(&self) -> i32 {
        if let Some(row) = self.sidebar.selected_row() {
            return row.index();
        }
        -1
    }
}
