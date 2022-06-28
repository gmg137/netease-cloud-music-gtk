//
// search_singer_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use glib::Sender;
use glib::{ParamFlags, ParamSpec, ParamSpecBoolean, ParamSpecInt, ParamSpecString, Value};
pub(crate) use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::SingerInfo;
use once_cell::sync::{Lazy, OnceCell};

use crate::{application::Action, model::SearchType, path::CACHE};
use std::cell::{Cell, RefCell};

glib::wrapper! {
    pub struct SearchSingerPage(ObjectSubclass<imp::SearchSingerPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl SearchSingerPage {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create SearchSingerPage")
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        self.imp().sender.set(sender).unwrap();
    }

    pub fn init_page(&self, keyword: String) {
        let offset = self.property::<i32>("offset");
        let imp = self.imp();
        let singer_grid = imp.singer_grid.get();
        for _ in 0..(offset / 5) {
            singer_grid.remove_row(1);
        }
        if offset < 5 {
            while let Some(child) = singer_grid.last_child() {
                singer_grid.remove(&child);
            }
        }
        self.set_property("offset", 0i32);
        self.set_property("keyword", keyword);
    }

    pub fn update_singer(&self, singer: Vec<SingerInfo>) {
        self.set_property("update", true);
        let offset = self.property::<i32>("offset");
        let singer_len = singer.len();
        let sender = self.imp().sender.get().unwrap();
        let singer_grid = self.imp().singer_grid.get();
        let mut row = (offset / 5) + 1;
        let mut col = 1;
        for si in singer {
            let mut path = CACHE.clone();
            path.push(format!("{}-singer.jpg", si.id));
            let avatar = adw::Avatar::new(140, Some(&si.name), true);
            if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(path) {
                let image = Image::from_pixbuf(Some(&pixbuf));
                if let Some(paintable) = image.paintable() {
                    avatar.set_custom_image(Some(&paintable));
                }
            }
            let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);
            boxs.append(&avatar);
            let label = gtk::Label::new(Some(&si.name));
            label.set_lines(2);
            label.set_margin_start(20);
            label.set_margin_end(20);
            label.set_width_chars(1);
            label.set_max_width_chars(1);
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_wrap(true);
            boxs.append(&label);
            singer_grid.attach(&boxs, col, row, 1, 1);
            col += 1;
            if col == 6 {
                col = 1;
                row += 1;
            }
            let gesture_click = GestureClick::new();
            avatar.add_controller(&gesture_click);
            let sender = sender.clone();
            gesture_click.connect_pressed(move |_, _, _, _| {
                sender.send(Action::ToSingerSongsPage(si.clone())).unwrap();
            });
        }
        self.set_property("offset", offset + singer_len as i32);
    }
}

impl Default for SearchSingerPage {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/search-singer-page.ui")]
    pub struct SearchSingerPage {
        #[template_child]
        pub singer_grid: TemplateChild<Grid>,

        update: Cell<bool>,
        offset: Cell<i32>,
        keyword: RefCell<String>,

        pub sender: OnceCell<Sender<Action>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchSingerPage {
        const NAME: &'static str = "SearchSingerPage";
        type Type = super::SearchSingerPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchSingerPage {
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
                _ => unimplemented!(),
            }
        }

        fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "update" => self.update.get().to_value(),
                "offset" => self.offset.get().to_value(),
                "keyword" => self.keyword.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for SearchSingerPage {}
    impl BoxImpl for SearchSingerPage {}
}

#[gtk::template_callbacks]
impl SearchSingerPage {
    #[template_callback]
    fn scrolled_edge_cb(&self, position: PositionType) {
        if self.property("update") {
            let sender = self.imp().sender.get().unwrap();
            if position == gtk::PositionType::Bottom {
                self.set_property("update", false);
                sender
                    .send(Action::Search(
                        self.property("keyword"),
                        SearchType::Singer,
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
