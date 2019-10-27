//
// mine.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use crate::app::Action;
use crate::musicapi::model::{SongInfo, SongList};
use crate::utils::*;
use crossbeam_channel::Sender;
use gtk::prelude::*;
use gtk::{
    Builder, Button, CellRendererText, Grid, Image, Label, ListBox, ListBoxRow, ListStore, Menu,
    MenuItem, ScrolledWindow, TreeView, TreeViewColumn,
};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
struct FmView {
    container: Grid,
    image: Image,
    like: Button,
    dislike: Button,
    play: Button,
    title: Label,
    singer: Label,
    nowplay: Rc<RefCell<Option<SongInfo>>>,
}

#[derive(Clone)]
struct UpView {
    container: Grid,
    dislike: Button,
    play: Button,
    refresh: Button,
    title: Label,
    number: Label,
}

#[derive(Clone)]
struct LowView {
    container: ScrolledWindow,
    popmenu: Menu,
    cc: MenuItem,
    tree: TreeView,
    store: ListStore,
}

#[derive(Clone)]
pub(crate) struct Mine {
    view: gtk::Box,
    sidebar: ListBox,
    fmview: FmView,
    upview: UpView,
    lowview: LowView,
    song_list: Vec<SongInfo>,
    sender: Sender<Action>,
}

impl Mine {
    pub(crate) fn new(builder: &Builder, sender: Sender<Action>) -> Self {
        let view: gtk::Box = builder
            .get_object("mine_view")
            .expect("无法获取 mine_view .");
        let sidebar: ListBox = builder
            .get_object("mine_listbox")
            .expect("无法获取 mine_listbox .");
        let container: Grid = builder
            .get_object("mine_fm_grid")
            .expect("无法获取 mine_fm_grid .");
        let image: Image = builder
            .get_object("mine_fm_image")
            .expect("无法获取 mine_fm_image .");
        let like: Button = builder
            .get_object("mine_fm_like_button")
            .expect("无法获取 mine_fm_like_button .");
        let dislike: Button = builder
            .get_object("mine_fm_dislike_button")
            .expect("无法获取 mine_fm_dislike_button .");
        let play: Button = builder
            .get_object("mine_fm_play_button")
            .expect("无法获取 mine_fm_play_button .");
        let title: Label = builder
            .get_object("mine_fm_title")
            .expect("无法获取 mine_fm_title .");
        let singer: Label = builder
            .get_object("mine_fm_singer")
            .expect("无法获取 mine_fm_singer .");
        let fmview = FmView {
            container,
            image,
            like,
            dislike,
            play,
            title,
            singer,
            nowplay: Rc::new(RefCell::new(None)),
        };
        let container: Grid = builder
            .get_object("mine_up_grid")
            .expect("无法获取 mine_up_grid .");
        let title: Label = builder
            .get_object("mine_up_title")
            .expect("无法获取 mine_up_title .");
        let number: Label = builder
            .get_object("mine_up_num")
            .expect("无法获取 min_up_num .");
        let play: Button = builder
            .get_object("mine_up_play_button")
            .expect("无法获取 mine_up_play_button .");
        let dislike: Button = builder
            .get_object("mine_up_del_button")
            .expect("无法获取 mine_up_del_button .");
        let refresh: Button = builder
            .get_object("mine_up_refresh_button")
            .expect("无法获取 mine_up_refresh_button .");
        let upview = UpView {
            container,
            title,
            number,
            dislike,
            play,
            refresh,
        };
        let container: ScrolledWindow = builder
            .get_object("mine_low_view")
            .expect("无法获取 mine_low_view .");
        let popmenu: Menu = builder
            .get_object("song_list_popup_menu")
            .expect("无法获取 song_list_popup_menu .");
        let cc: MenuItem = builder
            .get_object("mine_cancel_collection")
            .expect("无法获取 mine_cancel_collection .");
        let tree: TreeView = builder
            .get_object("mine_tree_view")
            .expect("无法获取 mine_tree_view .");
        let store: ListStore = ListStore::new(&[
            gtk::Type::U32,
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
        ]);

        let lowview = LowView {
            container,
            popmenu,
            cc,
            tree,
            store,
        };

        let s = Mine {
            view,
            sidebar,
            fmview,
            upview,
            lowview,
            song_list: vec![],
            sender: sender.clone(),
        };
        Self::init(&s);
        s
    }

    fn init(s: &Self) {
        let sender = s.sender.clone();
        sender.send(Action::RefreshMine).unwrap_or(());

        s.lowview.cc.connect_activate(move |_| {
            sender.send(Action::CancelCollection).unwrap_or(());
        });

        let sender = s.sender.clone();
        let listbox = s.sidebar.downgrade();
        let popmenu = s.lowview.popmenu.downgrade();
        s.lowview
            .tree
            .connect_button_press_event(move |tree, event| {
                if event.get_event_type() == gdk::EventType::ButtonPress && event.get_button() == 3
                {
                    if let Some(row) = listbox.upgrade().unwrap().get_selected_row() {
                        if row.get_index() == 2 {
                            popmenu.upgrade().unwrap().popup_easy(3, event.get_time());
                        }
                    }
                }
                if event.get_event_type() == gdk::EventType::DoubleButtonPress {
                    if let Some((model, iter)) = tree.get_selection().get_selected() {
                        let id = model.get_value(&iter, 0).get::<u32>().unwrap_or(0);
                        let name = model
                            .get_value(&iter, 1)
                            .get::<String>()
                            .unwrap_or("".to_owned());
                        let duration = model
                            .get_value(&iter, 2)
                            .get::<String>()
                            .unwrap_or("".to_owned());
                        let singer = model
                            .get_value(&iter, 3)
                            .get::<String>()
                            .unwrap_or("".to_owned());
                        let album = model
                            .get_value(&iter, 4)
                            .get::<String>()
                            .unwrap_or("".to_owned());
                        let pic_url = model
                            .get_value(&iter, 5)
                            .get::<String>()
                            .unwrap_or("".to_owned());
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
            sender
                .send(Action::RefreshMineViewInit(
                    row.as_ref().unwrap().get_index(),
                ))
                .unwrap_or(());
        });

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
            sender
                .send(Action::RefreshMineCurrentView())
                .unwrap_or(());
        });

        let sender = s.sender.clone();
        s.fmview.play.connect_clicked(move |_| {
            sender.send(Action::PlayerFm).unwrap_or(());
        });

        let sender = s.sender.clone();
        s.fmview.like.connect_clicked(move |_| {
            sender.send(Action::FmLike).unwrap_or(());
        });

        let sender = s.sender.clone();
        s.fmview.dislike.connect_clicked(move |_| {
            sender.send(Action::FmDislike).unwrap_or(());
        });
    }

    pub(crate) fn get_song_id(&self) -> Option<u32> {
        if let Some((model, iter)) = self.lowview.tree.get_selection().get_selected() {
            return model.get_value(&iter, 0).get::<u32>();
        }
        None
    }

    pub(crate) fn hide_all(&self) {
        self.view.hide();
    }

    pub(crate) fn show_fm(&self) {
        let one_row = self.sidebar.get_row_at_index(0).unwrap();
        self.sidebar.select_row(Some(&one_row));
        self.view.show_all();
        self.upview.container.hide();
        self.lowview.container.hide();
    }

    pub(crate) fn hide_fm(&self) {
        self.view.show_all();
        self.fmview.container.hide();
    }

    pub(crate) fn update_sidebar(&self, song_list: Vec<SongList>) {
        if let Some(one_row) = self.sidebar.get_row_at_index(0) {
            self.sidebar.select_row(Some(&one_row));
            self.sidebar.get_children()[2..].iter().for_each(|w| {
                self.sidebar.remove(w);
            });
        }
        let row = self.sidebar.get_row_at_index(3);
        if row.is_none() {
            song_list.iter().for_each(|sl| {
                let label = Label::new(Some(&sl.name[..]));
                label.set_halign(gtk::Align::Start);
                label.set_valign(gtk::Align::Fill);
                label.set_margin_start(18);
                label.set_ellipsize(pango::EllipsizeMode::End);
                label.set_max_width_chars(16);
                let row = ListBoxRow::new();
                row.set_property_height_request(58);
                row.add(&label);
                self.sidebar.insert(&row, -1);
            });
        }
        self.sidebar.show_all();
    }

    pub(crate) fn update_fm_view(&self, song_info: &SongInfo) {
        let mut image_path = format!(
            "{}/{}_p210.jpg",
            crate::CACHED_PATH.to_owned(),
            &song_info.id
        );
        download_img(&song_info.pic_url, &image_path, 210, 210);
        let path = format!("{}/{}_p210", crate::CACHED_PATH.to_owned(), &song_info.id);
        if create_round_avatar(path).is_ok() {
            image_path = format!(
                "{}/{}_p210.png",
                crate::CACHED_PATH.to_owned(),
                &song_info.id
            );
        }
        self.fmview.image.set_from_file(image_path);
        self.fmview.title.set_text(&song_info.name);
        self.fmview.singer.set_text(&song_info.singer);
    }

    pub(crate) fn update_up_view(&self, title: String) {
        self.upview.dislike.set_visible(false);
        self.upview.dislike.hide();
        if let Some(row) = self.sidebar.get_selected_row() {
            let index = row.get_index();
            if index > 2 {
                self.upview.dislike.set_visible(true);
                self.upview.dislike.show_all();
            }
        }
        self.lowview.store.clear();
        for c in self.lowview.tree.get_columns().iter() {
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
        title.set_property_xpad(20);
        title.set_property_xalign(0.0);
        title.set_property_yalign(0.5);
        title.set_property_height(48);
        column.pack_start(&title, true);
        column.add_attribute(&title, "text", 1);

        let duration = CellRendererText::new();
        duration.set_property_xpad(32);
        duration.set_property_xalign(0.0);
        column.pack_start(&duration, true);
        column.add_attribute(&duration, "text", 2);

        let singer = CellRendererText::new();
        singer.set_property_xpad(22);
        singer.set_property_xalign(0.0);
        column.pack_start(&singer, true);
        column.add_attribute(&singer, "text", 3);

        let album = CellRendererText::new();
        album.set_property_xpad(32);
        album.set_property_xalign(0.0);
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
                &[0, 1, 2, 3, 4, 5],
                &[
                    &song.id,
                    &song.name,
                    &song.duration,
                    &song.singer,
                    &song.album,
                    &song.pic_url,
                ],
            );
        });
    }

    pub(crate) fn play_all(&self) {
        create_player_list(&self.song_list, self.sender.clone(), true);
    }

    pub(crate) fn set_now_play(&self, si: SongInfo) {
        *self.fmview.nowplay.borrow_mut() = Some(si);
    }

    pub(crate) fn get_now_play(&self) -> Option<SongInfo> {
        self.fmview.nowplay.borrow().to_owned()
    }

    pub(crate) fn play_fm(&self) {
        let sender = self.sender.clone();
        self.fmview.nowplay.borrow().clone().map(|si| {
            sender
                .send(Action::PlayerInit(si.to_owned(), PlayerTypes::Fm))
                .unwrap_or(());
        });
    }

    pub(crate) fn get_selected_row_id(&self) -> i32 {
        if let Some(row) = self.sidebar.get_selected_row() {
            return row.get_index();
        }
        -1
    }
}
