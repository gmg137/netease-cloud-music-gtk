//
// search_songlist_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use glib::Sender;
use glib::{ParamSpec, ParamSpecBoolean, ParamSpecEnum, ParamSpecInt, ParamSpecString, Value};
pub(crate) use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::SongList;
use once_cell::sync::{Lazy, OnceCell};

use crate::{
    application::Action,
    gui::SongListGridItem,
    model::{SearchResult, SearchType},
};
use std::cell::{Cell, RefCell};
use std::sync::Arc;

glib::wrapper! {
    pub struct SearchSongListPage(ObjectSubclass<imp::SearchSongListPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl SearchSongListPage {
    pub fn new() -> Self {
        glib::Object::new(&[])
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn init_page(&self, keyword: &str, search_type: SearchType) {
        let imp = self.imp();
        let songlist_grid = imp.songlist_grid.get();
        SongListGridItem::view_clear(songlist_grid);

        self.set_property("offset", 0i32);
        self.set_property("keyword", keyword);
        self.set_property("search-type", search_type);
    }

    pub fn update_songlist(&self, song_list: &[SongList]) {
        let sender = self.imp().sender.get().unwrap();
        let grid = self.imp().songlist_grid.get();
        self.set_property("update", true);
        let offset = self.property::<i32>("offset");
        let song_list_len = song_list.len();

        let show_author = matches!(
            self.property::<SearchType>("search-type"),
            SearchType::Album | SearchType::AllAlbums | SearchType::LikeAlbums
        );

        SongListGridItem::view_update_songlist(grid, song_list, 140, show_author, sender);

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
        #[template_child(id = "songlist_grid")]
        pub songlist_grid: TemplateChild<gtk::GridView>,

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
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_widget_name("songlist_page");
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("update").build(),
                    ParamSpecInt::builder("offset").build(),
                    ParamSpecString::builder("keyword")
                        .default_value(None)
                        .build(),
                    ParamSpecEnum::builder("search-type", SearchType::default())
                        .explicit_notify()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
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

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
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
        let offset = self.property::<i32>("offset");
        if self.property("update") && offset % 50 == 0 {
            let sender = self.imp().sender.get().unwrap();
            if position == gtk::PositionType::Bottom {
                self.set_property("update", false);
                let s = glib::SendWeakRef::from(self.downgrade());
                sender
                    .send(Action::Search(
                        self.property("keyword"),
                        self.property("search-type"),
                        offset as u16,
                        50,
                        Arc::new(move |sls| {
                            if let Some(s) = s.upgrade() {
                                if let SearchResult::SongLists(sls) = sls {
                                    s.update_songlist(&sls);
                                }
                            }
                        }),
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
    #[template_callback]
    fn grid_activate_cb(&self, pos: u32) {
        let search_type = self.property::<SearchType>("search-type");
        let sender = self.imp().sender.get().unwrap();

        let item = SongListGridItem::view_item_at_pos(self.imp().songlist_grid.get(), pos).unwrap();
        match search_type {
            SearchType::Album | SearchType::AllAlbums | SearchType::LikeAlbums => {
                sender.send(Action::ToAlbumPage(item.into())).unwrap();
            }
            SearchType::SongList | SearchType::TopPicks | SearchType::LikeSongList => {
                sender.send(Action::ToSongListPage(item.into())).unwrap();
            }
            _ => (),
        }
    }
}
