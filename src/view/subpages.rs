//
// subpages.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the MIT license.
//
use crate::{
    app::Action,
    data::MusicData,
    musicapi::model::SongInfo,
    utils::{create_player_list, PlayerTypes},
};
use async_std::task;
use gdk_pixbuf::{InterpType, Pixbuf};
use glib::Sender;
use gtk::{
    prelude::*, Builder, Button, CellRendererText, Grid, Image, Label, ListStore, ScrolledWindow, TreeView,
    TreeViewColumn,
};
use pango::EllipsizeMode;

#[derive(Clone)]
pub(crate) struct Subpages {
    overview: Overview,
    scrolled: ScrolledWindow,
    tree: TreeView,
    store: ListStore,
    song_list: Vec<SongInfo>,
    song_list_id: u64,
    sender: Sender<Action>,
}

#[derive(Clone)]
pub(crate) struct Overview {
    grid: Grid,
    pic: Image,
    album: Label,
    num: Label,
    like: Button,
    play: Button,
}

impl Subpages {
    pub(crate) fn new(builder: &Builder, sender: Sender<Action>) -> Self {
        let grid: Grid = builder.object("subpages_grid").expect("无法获取 subpages_grid .");
        let pic: Image = builder
            .object("subpages_album_image")
            .expect("无法获取 subpages_album_image .");
        let album: Label = builder
            .object("subpages_album_name")
            .expect("无法获取 subpages_album_name .");
        let num: Label = builder
            .object("subpages_song_num")
            .expect("无法获取 subpages_song_num .");
        let like: Button = builder
            .object("subpages_like_button")
            .expect("无法获取 subpages_like_button .");
        let play: Button = builder
            .object("subpages_play_button")
            .expect("无法获取 subpages_play_button .");
        let scrolled: ScrolledWindow = builder
            .object("subpages_scrolled_window")
            .expect("无法获取 subpages_scrolled_window.");
        let overview = Overview {
            grid,
            pic,
            album,
            num,
            like,
            play,
        };
        let tree: TreeView = builder
            .object("subpages_tree_view")
            .expect("无法获取 subpages_tree_view .");
        let store: ListStore = ListStore::new(&[
            glib::Type::U64,
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let s = Subpages {
            overview,
            scrolled,
            tree,
            store,
            song_list: vec![],
            song_list_id: 0,
            sender,
        };
        Self::init(&s);
        s
    }

    fn init(s: &Self) {
        let sender = s.sender.clone();
        s.tree.connect_button_press_event(move |tree, event| {
            if event.event_type() == gdk::EventType::DoubleButtonPress {
                if let Some((model, iter)) = tree.selection().selected() {
                    let id = model.value(&iter, 0).get::<u64>().unwrap_or(0);
                    let name = model
                        .value(&iter, 1)
                        .get::<String>()
                        .unwrap_or_else(|_| "".to_owned());
                    let duration = model
                        .value(&iter, 2)
                        .get::<String>()
                        .unwrap_or_else(|_| "".to_owned());
                    let singer = model
                        .value(&iter, 3)
                        .get::<String>()
                        .unwrap_or_else(|_| "".to_owned());
                    let album = model
                        .value(&iter, 4)
                        .get::<String>()
                        .unwrap_or_else(|_| "".to_owned());
                    let pic_url = model
                        .value(&iter, 5)
                        .get::<String>()
                        .unwrap_or_else(|_| "".to_owned());
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
        s.overview.play.connect_clicked(move |_| {
            sender.send(Action::PlayerSubpages).unwrap_or(());
        });

        // 收藏歌单
        let sender = s.sender.clone();
        s.overview.like.connect_clicked(move |_| {
            sender.send(Action::LikeSongList).unwrap_or(());
        });

        // 检测是否滚动到底部边缘
        let sender = s.sender.clone();
        s.scrolled.connect_edge_overshot(move |_, position| {
            if position == gtk::PositionType::Bottom {
                sender.send(Action::AppendSearch).unwrap_or(());
            }
        });
    }

    pub(crate) fn update_up_view(&mut self, id: u64, name: String, image_path: String) {
        self.song_list_id = id;
        self.overview.grid.show();
        self.store.clear();
        for c in self.tree.columns().iter() {
            self.tree.remove_column(c);
        }
        self.tree.set_model(Some(&self.store));
        if name.is_empty() && image_path.is_empty() {
            self.overview.grid.hide();
            return;
        }
        if name.starts_with("search:") {
            self.overview.grid.hide();
        }
        if let Ok(image) = Pixbuf::from_file(&image_path) {
            let image = image.scale_simple(140, 140, InterpType::Bilinear);
            self.overview.pic.set_from_pixbuf(image.as_ref());
        };
        self.overview.album.set_label(&name);
        self.overview.num.set_label("0 首");
        // 每次打开时回到页面顶部, 此设置会导致滚动条消失
        //if let Some(adj) = self.scrolled.get_vadjustment() {
        //adj.set_value(0.0);
        //}
    }

    pub(crate) fn update_low_view(&mut self, song_list: Vec<SongInfo>) {
        let column = TreeViewColumn::new();
        column.set_visible(false);
        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        let id = CellRendererText::new();
        column.pack_start(&id, true);
        column.add_attribute(&id, "text", 0);
        self.tree.append_column(&column);

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
        self.tree.append_column(&column);

        let column = TreeViewColumn::new();
        column.set_visible(false);
        column.set_sizing(gtk::TreeViewColumnSizing::Fixed);
        let url = CellRendererText::new();
        column.pack_start(&url, true);
        column.add_attribute(&url, "text", 5);
        self.tree.append_column(&column);

        self.song_list = song_list.to_owned();
        let num = format!("{} 首", song_list.len());
        self.overview.num.set_label(&num);
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

    pub(crate) fn append_low_view(&mut self, song_list: Vec<SongInfo>) {
        let mut song_list_old = self.song_list.to_owned();
        let mut song_list_new = song_list.to_owned();
        song_list_old.append(&mut song_list_new);
        self.song_list = song_list_old;
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

    // 显示收藏按钮
    pub(crate) fn show_like(&self, show: bool) {
        self.overview.like.set_visible(false);
        self.overview.like.hide();
        if show {
            self.overview.like.set_visible(true);
            self.overview.like.show_all();
        }
    }

    // 获取歌单 id
    pub(crate) fn get_song_list_id(&self) -> u64 {
        self.song_list_id
    }

    // 获取搜索数据
    // return: (搜索关键词,已加载歌曲数)
    pub(crate) fn get_search_data(&self) -> Option<(String, usize)> {
        let text = self.overview.album.text().to_string();
        let num = self.song_list.len();
        if let Some(key) = text.strip_prefix("search:") {
            Some((key.to_owned(), num))
        } else {
            None
        }
    }
}
