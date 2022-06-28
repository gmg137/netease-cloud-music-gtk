//
// search_songlist_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use glib::Sender;
use glib::{
    ParamFlags, ParamSpec, ParamSpecBoolean, ParamSpecEnum, ParamSpecInt, ParamSpecString, Value,
};
pub(crate) use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::SongList;
use once_cell::sync::{Lazy, OnceCell};

use crate::{application::Action, model::SearchType, path::CACHE};
use std::cell::{Cell, RefCell};

glib::wrapper! {
    pub struct SearchSongListPage(ObjectSubclass<imp::SearchSongListPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl SearchSongListPage {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create SearchSongListPage")
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn init_page(&self, keyword: String, search_type: SearchType) {
        let imp = self.imp();
        let offset = self.property::<i32>("offset");
        let songlist_grid = imp.songlist_grid.get();
        for _ in 0..(offset / 5) {
            songlist_grid.remove_row(1);
        }
        while let Some(child) = songlist_grid.last_child() {
            songlist_grid.remove(&child);
        }
        self.set_property("offset", 0i32);
        self.set_property("keyword", keyword);
        self.set_property("search-type", search_type);
    }

    pub fn update_songlist(&self, song_list: Vec<SongList>) {
        self.set_property("update", true);
        let offset = self.property::<i32>("offset");
        let song_list_len = song_list.len();
        let sender = self.imp().sender.get().unwrap();
        let songlist_grid = self.imp().songlist_grid.get();
        let mut row = (offset / 5) + 1;
        let mut col = 1;
        for sl in song_list {
            let mut path = CACHE.clone();
            path.push(format!("{}-songlist.jpg", sl.id));
            let image = gtk::Image::from_file(path);
            let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
            image.set_pixel_size(140);
            let frame = gtk::Frame::new(None);
            frame.set_halign(gtk::Align::Center);
            frame.set_child(Some(&image));
            boxs.append(&frame);
            let label = gtk::Label::new(Some(&sl.name));
            label.set_lines(2);
            label.set_margin_start(20);
            label.set_margin_end(20);
            label.set_width_chars(1);
            label.set_max_width_chars(1);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_wrap(true);
            boxs.append(&label);
            songlist_grid.attach(&boxs, col, row, 1, 1);
            col += 1;
            if col == 6 {
                col = 1;
                row += 1;
            }
            let gesture_click = GestureClick::new();
            image.add_controller(&gesture_click);
            let sender = sender.clone();
            let search_type = self.property::<SearchType>("search-type");
            gesture_click.connect_pressed(move |_, _, _, _| match search_type {
                SearchType::Album => {
                    sender.send(Action::ToAlbumPage(sl.clone())).unwrap();
                }
                SearchType::AllAlbums => {
                    sender.send(Action::ToAlbumPage(sl.clone())).unwrap();
                }
                SearchType::LikeAlbums => {
                    sender.send(Action::ToAlbumPage(sl.clone())).unwrap();
                }
                SearchType::SongList => {
                    sender.send(Action::ToSongListPage(sl.clone())).unwrap();
                }
                SearchType::TopPicks => {
                    sender.send(Action::ToSongListPage(sl.clone())).unwrap();
                }
                SearchType::LikeSongList => {
                    sender.send(Action::ToSongListPage(sl.clone())).unwrap();
                }
                _ => (),
            });
        }
        self.set_property("offset", offset + song_list_len as i32);
    }
}

impl Default for SearchSongListPage {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/search-songlist-page.ui")]
    pub struct SearchSongListPage {
        #[template_child]
        pub songlist_grid: TemplateChild<Grid>,

        update: Cell<bool>,
        offset: Cell<i32>,
        keyword: RefCell<String>,
        search_type: Cell<SearchType>,

        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchSongListPage {
        const NAME: &'static str = "SearchSongListPage";
        type Type = super::SearchSongListPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchSongListPage {
        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::new(
                        // Name
                        "update",
                        // Nickname
                        "update",
                        // Short description
                        "Used to determine if the page is updated when scrolling to the bottom.",
                        // Default value
                        false,
                        // The property can be read and written to
                        ParamFlags::READWRITE,
                    ),
                    ParamSpecInt::new(
                        // Name
                        "offset",
                        // Nickname
                        "offset",
                        // Short description
                        "offset",
                        // Minimum value
                        i32::MIN,
                        // Maximum value
                        i32::MAX,
                        // Default value
                        0,
                        // The property can be read and written to
                        ParamFlags::READWRITE,
                    ),
                    ParamSpecString::new(
                        // Name
                        "keyword",
                        // Nickname
                        "keyword",
                        // Short description
                        "Search keyword",
                        // Default value
                        None,
                        // The property can be read and written to
                        ParamFlags::READWRITE,
                    ),
                    ParamSpecEnum::new(
                        // Name
                        "search-type",
                        // Nickname
                        "search-type",
                        // Short description
                        "search type",
                        // Enum type
                        SearchType::static_type(),
                        // Default value
                        SearchType::default() as i32,
                        // The property can be read and written to
                        ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "update" => {
                    let update = value.get().expect("The value needs to be of type `bool`.");
                    self.update.replace(update);
                }
                "offset" => {
                    let offset = value.get().expect("The value needs to be of type `i32`.");
                    self.offset.replace(offset);
                }
                "keyword" => {
                    let keyword = value.get().unwrap();
                    self.keyword.replace(keyword);
                }
                "search-type" => {
                    let input_type = value
                        .get()
                        .expect("The value needs to be of type `SearchType`.");
                    self.search_type.replace(input_type);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "update" => self.update.get().to_value(),
                "offset" => self.offset.get().to_value(),
                "keyword" => self.keyword.borrow().to_value(),
                "search-type" => self.search_type.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for SearchSongListPage {}
    impl BoxImpl for SearchSongListPage {}
}

#[gtk::template_callbacks]
impl SearchSongListPage {
    #[template_callback]
    fn scrolled_edge_cb(&self, position: PositionType) {
        if self.property("update") {
            let sender = self.imp().sender.get().unwrap();
            if position == gtk::PositionType::Bottom {
                self.set_property("update", false);
                sender
                    .send(Action::Search(
                        self.property("keyword"),
                        self.property("search-type"),
                        self.property::<i32>("offset") as u16,
                        50,
                    ))
                    .unwrap_or(());
                sender
                    .send(Action::AddToast(gettextrs::gettext(
                        "Loading more content...",
                    )))
                    .unwrap();
            }
        }
    }
}
