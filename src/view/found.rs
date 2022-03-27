//
// found.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::{
    app::Action,
    data::MusicData,
    musicapi::model::SongInfo,
    utils::{create_player_list, PlayerTypes},
};
use async_std::task;
use glib::Sender;
use gtk::{prelude::*, Builder, Button, CellRendererText, Label, ListBox, ListStore, TreeView, TreeViewColumn};
use pango::EllipsizeMode;

#[derive(Clone)]
pub(crate) struct Found {
    sidebar: ListBox,
    title: Label,
    number: Label,
    play: Button,
    treeview: TreeView,
    store: ListStore,
    song_list: Vec<SongInfo>,
    sender: Sender<Action>,
}

impl Found {
    pub(crate) fn new(builder: &Builder, sender: Sender<Action>) -> Self {
        let sidebar: ListBox = builder.object("found_listbox").expect("无法获取 found_listbox .");
        let title: Label = builder
            .object("found_songs_title")
            .expect("无法获取 found_songs_title .");
        let number: Label = builder.object("found_songs_num").expect("无法获取 found_songs_num .");
        let play: Button = builder
            .object("found_play_button")
            .expect("无法获取 found_play_button .");
        let treeview: TreeView = builder.object("found_tree_view").expect("无法获取 found_tree_view .");
        let store: ListStore = ListStore::new(&[
            glib::Type::U64,
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
        ]);

        let s = Found {
            sidebar,
            title,
            number,
            play,
            treeview,
            store,
            song_list: vec![],
            sender,
        };
        Self::init(&s);
        s
    }

    fn init(s: &Self) {
        if let Some(one_row) = s.sidebar.row_at_index(0) {
            s.sidebar.select_row(Some(&one_row));
        }
        let sender = s.sender.clone();
        s.treeview.connect_button_press_event(move |tree, event| {
            if event.event_type() == gdk::EventType::DoubleButtonPress {
                if let Some((model, iter)) = tree.selection().selected() {
                    let id = model.value(&iter, 0).get::<u64>().unwrap_or(0);
                    let name = model.value(&iter, 1).get::<String>().unwrap_or_else(|_| String::new());
                    let duration = model.value(&iter, 2).get::<String>().unwrap_or_else(|_| String::new());
                    let singer = model.value(&iter, 3).get::<String>().unwrap_or_else(|_| String::new());
                    let album = model.value(&iter, 4).get::<String>().unwrap_or_else(|_| String::new());
                    let pic_url = model.value(&iter, 5).get::<String>().unwrap_or_else(|_| String::new());
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
                sender
                    .send(Action::RefreshFoundViewInit(row.index() as u8))
                    .unwrap_or(());
            }
        });

        let sender = s.sender.clone();
        s.play.connect_clicked(move |_| {
            sender.send(Action::PlayerFound).unwrap_or(());
        });
        s.sender.send(Action::RefreshFoundViewInit(0)).unwrap_or(());
    }

    pub(crate) fn update_up_view(&self, title: String) {
        self.store.clear();
        for c in self.treeview.columns().iter() {
            self.treeview.remove_column(c);
        }
        self.treeview.set_model(Some(&self.store));
        self.title.set_text(&title);
        self.number.set_text("0 首");
    }

    pub(crate) fn update_low_view(&mut self, song_list: Vec<SongInfo>) {
        let column = TreeViewColumn::new();
        column.set_visible(false);
        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        let id = CellRendererText::new();
        column.pack_start(&id, true);
        column.add_attribute(&id, "text", 0);
        self.treeview.append_column(&column);

        let column = TreeViewColumn::new();
        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        let title = CellRendererText::new();
        title.set_xpad(20);
        title.set_xalign(0.0);
        title.set_yalign(0.5);
        title.set_height(48);
        title.set_ellipsize(EllipsizeMode::End);
        column.pack_start(&title, true);
        column.add_attribute(&title, "text", 1);

        let duration = CellRendererText::new();
        duration.set_xpad(32);
        duration.set_xalign(0.0);
        column.pack_start(&duration, true);
        column.add_attribute(&duration, "text", 2);

        let singer = CellRendererText::new();
        singer.set_xpad(22);
        singer.set_xalign(0.0);
        singer.set_ellipsize(EllipsizeMode::End);
        column.pack_start(&singer, true);
        column.add_attribute(&singer, "text", 3);

        let album = CellRendererText::new();
        album.set_xpad(32);
        album.set_xalign(0.0);
        album.set_ellipsize(EllipsizeMode::End);
        column.pack_start(&album, true);
        column.add_attribute(&album, "text", 4);
        self.treeview.append_column(&column);

        let column = TreeViewColumn::new();
        column.set_visible(false);
        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        let url = CellRendererText::new();
        column.pack_start(&url, true);
        column.add_attribute(&url, "text", 5);
        self.treeview.append_column(&column);

        self.song_list = song_list.to_owned();
        let num = format!("{} 首", song_list.len());
        self.number.set_label(&num);
        song_list.iter().for_each(|song| {
            self.store.insert_with_values(
                None,
                &[
                    (0, &song.id),
                    (1, &song.name),
                    (2, &song.duration),
                    (3, &song.singer),
                    (4, &song.album),
                    (5, &song.pic_url),
                ],
            );
        });
    }

    pub(crate) fn play_all(&self) {
        let song_list = self.song_list.clone();
        let sender = self.sender.clone();
        sender.send(Action::PlayerTypes(PlayerTypes::Song)).unwrap_or(());
        task::spawn(async move {
            let mut api = MusicData::new().await;
            create_player_list(&mut api, &song_list, sender, true, false).await.ok()
        });
    }
}
