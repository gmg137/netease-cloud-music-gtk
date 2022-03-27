//
// list.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::{app::Action, data::MusicData, musicapi::model::SongInfo, utils::*};
use async_std::{
    sync::{Arc, Mutex},
    task,
};
use glib::Sender;
use gtk::{prelude::*, Builder, Button, CellRendererText, Label, ListStore, Menu, MenuItem, TreeView, TreeViewColumn};
use pango::EllipsizeMode;

#[derive(Clone)]
struct UpView {
    dislike: Button,
    play: Button,
    refresh: Button,
    title: Label,
    number: Label,
}

#[derive(Clone)]
pub struct LowView {
    pub(crate) popmenu: Menu,
    cc: MenuItem,
    pub(crate) tree: TreeView,
    store: ListStore,
}

#[derive(Clone)]
pub(crate) struct ListView {
    upview: UpView,
    pub(crate) lowview: LowView,
    song_list: Vec<SongInfo>,
    sender: Sender<Action>,
    music_data: Arc<Mutex<MusicData>>,
}

impl ListView {
    pub(crate) fn new(
        mine_login_list_builder: &Builder,
        sender: Sender<Action>,
        music_data: Arc<Mutex<MusicData>>,
    ) -> Self {
        let title: Label = mine_login_list_builder
            .object("mine_up_title")
            .expect("无法获取 mine_up_title .");
        let number: Label = mine_login_list_builder
            .object("mine_up_num")
            .expect("无法获取 min_up_num .");
        let play: Button = mine_login_list_builder
            .object("mine_up_play_button")
            .expect("无法获取 mine_up_play_button .");
        let dislike: Button = mine_login_list_builder
            .object("mine_up_del_button")
            .expect("无法获取 mine_up_del_button .");
        let refresh: Button = mine_login_list_builder
            .object("mine_up_refresh_button")
            .expect("无法获取 mine_up_refresh_button .");
        let upview = UpView {
            title,
            number,
            dislike,
            play,
            refresh,
        };
        let popmenu: Menu = mine_login_list_builder
            .object("song_list_popup_menu")
            .expect("无法获取 song_list_popup_menu .");
        let cc: MenuItem = mine_login_list_builder
            .object("mine_cancel_collection")
            .expect("无法获取 mine_cancel_collection .");
        let tree: TreeView = mine_login_list_builder
            .object("mine_tree_view")
            .expect("无法获取 mine_tree_view .");
        let store: ListStore = ListStore::new(&[
            glib::Type::U64,
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
        ]);

        let lowview = LowView {
            popmenu,
            cc,
            tree,
            store,
        };

        let s = ListView {
            upview,
            lowview,
            song_list: vec![],
            sender,
            music_data,
        };
        Self::init(&s);
        s
    }

    fn init(s: &Self) {
        // 取消收藏
        let sender = s.sender.clone();
        s.lowview.cc.connect_activate(move |_| {
            sender.send(Action::CancelCollection).unwrap_or(());
        });

        // 播放全部
        let sender = s.sender.clone();
        s.upview.play.connect_clicked(move |_| {
            sender.send(Action::PlayerMine).unwrap_or(());
        });

        // 删除歌单
        let sender = s.sender.clone();
        s.upview.dislike.connect_clicked(move |_| {
            sender.send(Action::DisLikeSongList).unwrap_or(());
        });

        // 刷新歌单
        let sender = s.sender.clone();
        s.upview.refresh.connect_clicked(move |_| {
            sender.send(Action::RefreshMineCurrentView()).unwrap_or(());
        });
    }

    pub(crate) fn get_song_id(&self) -> Option<u64> {
        if let Some((model, iter)) = self.lowview.tree.selection().selected() {
            return model.value(&iter, 0).get::<u64>().ok();
        }
        None
    }

    pub(crate) fn update_up_view(&self, title: String, sidebar_id: i32) {
        self.upview.dislike.set_visible(false);
        self.upview.dislike.hide();
        if sidebar_id > 3 {
            self.upview.dislike.set_visible(true);
            self.upview.dislike.show_all();
        }
        self.lowview.store.clear();
        for c in self.lowview.tree.columns().iter() {
            self.lowview.tree.remove_column(c);
        }
        self.lowview.tree.set_model(Some(&self.lowview.store));
        self.upview.title.set_text(&title);
        self.upview.number.set_text("0 首");
    }

    pub(crate) fn update_low_view(&mut self, song_list: Vec<SongInfo>) {
        let column = TreeViewColumn::new();
        column.set_visible(false);
        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        let id = CellRendererText::new();
        column.pack_start(&id, true);
        column.add_attribute(&id, "text", 0);
        self.lowview.tree.append_column(&column);

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
        self.lowview.tree.append_column(&column);

        let column = TreeViewColumn::new();
        column.set_visible(false);
        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        let url = CellRendererText::new();
        column.pack_start(&url, true);
        column.add_attribute(&url, "text", 5);
        self.lowview.tree.append_column(&column);

        self.song_list = song_list.to_owned();
        let num = format!("{} 首", song_list.len());
        self.upview.number.set_label(&num);
        song_list.iter().for_each(|song| {
            self.lowview.store.insert_with_values(
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
        let data = self.music_data.clone();
        task::spawn(async move {
            let mut api = data.lock().await;
            create_player_list(&mut api, &song_list, sender, true, false).await.ok()
        });
    }
}
