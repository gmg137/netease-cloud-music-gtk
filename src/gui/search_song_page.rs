//
// search_song_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use glib::Sender;
use glib::{
    ParamFlags, ParamSpec, ParamSpecBoolean, ParamSpecEnum, ParamSpecInt, ParamSpecString, Value,
};
pub(crate) use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::SongInfo;
use once_cell::sync::{Lazy, OnceCell};

use crate::application::Action;
use crate::model::SearchType;
use gettextrs::gettext;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use super::SonglistRow;

glib::wrapper! {
    pub struct SearchSongPage(ObjectSubclass<imp::SearchSongPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl SearchSongPage {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create SearchSongPage")
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn init_page(&self, keyword: String, search_type: SearchType) {
        let imp = self.imp();
        imp.playlist.replace(Vec::new());
        let title_clamp = imp.title_clamp.get();
        match search_type {
            SearchType::DailyRec
            | SearchType::Heartbeat
            | SearchType::CloudDisk
            | SearchType::Fm => {
                title_clamp.set_visible(true);
                imp.title_label.set_label(&keyword);
            }
            _ => {
                title_clamp.set_visible(false);
            }
        }
        self.set_property("offset", 0);
        self.set_property("keyword", keyword);
        self.set_property("search-type", search_type);
        let listbox = imp.listbox.get();
        while let Some(child) = listbox.last_child() {
            listbox.remove(&child);
        }
    }

    pub fn update_songs(&self, sis: Vec<SongInfo>) {
        self.set_property("update", true);
        let offset = self.property::<i32>("offset") + sis.len() as i32;
        self.set_property("offset", offset);
        let imp = self.imp();
        let mut playlist = sis.clone();
        (&*imp.playlist).borrow_mut().append(&mut playlist);
        imp.num_label.get().set_label(&gettext!("{} songs", offset));
        let sender = imp.sender.get().unwrap();
        let listbox = imp.listbox.get();
        sis.into_iter().for_each(|si| {
            let row = SonglistRow::new();
            row.set_tooltip_text(Some(&si.name));

            row.set_name(&si.name);
            row.set_singer(&si.singer);
            row.set_album(&si.album);
            row.set_duration(&si.duration);

            let sender = sender.clone();
            row.connect_activate(move |row| {
                row.switch_image(true);
                sender.send(Action::AddPlay(si.clone())).unwrap();
            });
            listbox.append(&row);
        });
    }
}

impl Default for SearchSongPage {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/search-song-page.ui")]
    pub struct SearchSongPage {
        #[template_child]
        pub title_clamp: TemplateChild<adw::Clamp>,
        #[template_child]
        pub title_label: TemplateChild<Label>,
        #[template_child]
        pub num_label: TemplateChild<Label>,
        #[template_child]
        pub listbox: TemplateChild<ListBox>,
        update: Cell<bool>,
        offset: Cell<i32>,
        keyword: RefCell<String>,
        search_type: Cell<SearchType>,

        pub playlist: Rc<RefCell<Vec<SongInfo>>>,
        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchSongPage {
        const NAME: &'static str = "SearchSongPage";
        type Type = super::SearchSongPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchSongPage {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let select_row = Rc::new(RefCell::new(-1));
            self.listbox.connect_row_activated(move |list, row| {
                let index;
                {
                    index = *select_row.borrow();
                }
                if index != -1 && index != row.index() {
                    *select_row.borrow_mut() = row.index();
                    if let Some(row) = list.row_at_index(index) {
                        let row = row.downcast::<SonglistRow>().unwrap();
                        row.switch_image(false);
                    }
                } else {
                    *select_row.borrow_mut() = row.index();
                }
            });
        }

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
    impl WidgetImpl for SearchSongPage {}
    impl BoxImpl for SearchSongPage {}
}

#[gtk::template_callbacks]
impl SearchSongPage {
    #[template_callback]
    fn scrolled_edge_cb(&self, position: PositionType) {
        match self.property::<SearchType>("search-type") {
            SearchType::DailyRec => return,
            SearchType::Heartbeat => return,
            SearchType::CloudDisk => return,
            _ => (),
        }
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
                    .send(Action::AddToast(gettext("Loading more content...")))
                    .unwrap();
            }
        }
    }

    #[template_callback]
    fn play_button_clicked_cb(&self) {
        let sender = self.imp().sender.get().unwrap();
        if !self.imp().playlist.borrow().is_empty() {
            let playlist = &*self.imp().playlist.borrow();
            sender.send(Action::AddPlayList(playlist.clone())).unwrap();
        } else {
            sender
                .send(Action::AddToast(gettext("This is an empty song list！")))
                .unwrap();
        }
    }
}
