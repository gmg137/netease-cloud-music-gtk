//
// subpages.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the MIT license.
//
use crate::app::Action;
use crate::musicapi::model::SongInfo;
use crate::utils::{create_player_list, PlayerTypes};
use crossbeam_channel::Sender;
use gtk::prelude::*;
use gtk::{
    Builder, Button, CellRendererText, Grid, Image, Label, ListStore, TreeView, TreeViewColumn,
};

#[derive(Clone)]
pub(crate) struct Subpages {
    overview: Overview,
    tree: TreeView,
    store: ListStore,
    song_list: Vec<SongInfo>,
    sender: Sender<Action>,
}

#[derive(Clone)]
pub(crate) struct Overview {
    grid: Grid,
    pic: Image,
    album: Label,
    num: Label,
    play: Button,
}

impl Subpages {
    pub(crate) fn new(builder: &Builder, sender: Sender<Action>) -> Self {
        let grid: Grid = builder
            .get_object("subpages_grid")
            .expect("无法获取 subpages_grid .");
        let pic: Image = builder
            .get_object("subpages_album_image")
            .expect("无法获取 subpages_album_image .");
        let album: Label = builder
            .get_object("subpages_album_name")
            .expect("无法获取 subpages_album_name .");
        let num: Label = builder
            .get_object("subpages_song_num")
            .expect("无法获取 subpages_song_num .");
        let play: Button = builder
            .get_object("subpages_play_button")
            .expect("无法获取 subpages_play_button .");
        let overview = Overview {
            grid,
            pic,
            album,
            num,
            play,
        };
        let tree: TreeView = builder
            .get_object("subpages_tree_view")
            .expect("无法获取 subpages_tree_view .");
        let store: ListStore = ListStore::new(&[
            gtk::Type::U32,
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
            String::static_type(),
        ]);
        let s = Subpages {
            overview,
            tree,
            store,
            song_list: vec![],
            sender,
        };
        Self::init(&s);
        s
    }

    fn init(s: &Self) {
        let sender = s.sender.clone();
        s.tree.connect_button_press_event(move |tree, event| {
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
        s.overview.play.connect_clicked(move |_| {
            sender.send(Action::PlayerSubpages).unwrap_or(());
        });
    }

    pub(crate) fn update_up_view(&self, name: String, image_path: String) {
        self.overview.grid.show();
        self.store.clear();
        for c in self.tree.get_columns().iter() {
            self.tree.remove_column(c);
        }
        self.tree.set_model(Some(&self.store));
        if name.is_empty() && image_path.is_empty() {
            self.overview.grid.hide();
        }
        self.overview.pic.set_from_file(&image_path);
        self.overview.album.set_label(&name);
        self.overview.num.set_label("0 首");
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
}
