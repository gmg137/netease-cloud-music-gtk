//
// search_grid_item.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use glib::Sender;
use glib::{ParamSpec, ParamSpecObject, ParamSpecString, ParamSpecUInt64, Value};
use gtk::{glib, prelude::*, subclass::prelude::*, *};
use ncm_api::SongList;
use once_cell::sync::Lazy;

use crate::{application::Action, model::ImageDownloadImpl};
use std::cell::{Cell, RefCell};

glib::wrapper! {
    pub struct SongListGridItem(ObjectSubclass<imp::SongListGridItem>);
}

impl From<SongListGridItem> for SongList {
    fn from(item: SongListGridItem) -> SongList {
        SongList {
            id: item.property::<u64>("id"),
            name: item.property::<String>("name"),
            cover_img_url: item.property::<String>("pic-url"),
            author: item.property::<String>("author"),
        }
    }
}

impl SongListGridItem {
    pub fn new(sl: &SongList, sender: &Sender<Action>) -> Self {
        let icon = Image::from_icon_name("image-missing-symbolic");

        let s: Self = glib::Object::builder()
            .property("id", &sl.id)
            .property("name", &sl.name)
            .property("pic-url", &sl.cover_img_url)
            .property("author", &sl.author)
            .property("icon", &icon)
            .build();

        let mut path = crate::path::CACHE.clone();
        path.push(format!("{}-songlist.jpg", sl.id));

        // download cover
        if !path.exists() {
            icon.set_from_net(sl.cover_img_url.to_owned(), path, (140, 140), sender);
        } else {
            icon.set_from_file(Some(&path));
        }
        s
    }

    fn create(pic_size: i32) -> (Box, Image, Label, Label) {
        let boxs = Box::new(Orientation::Vertical, 0);

        let image = Image::builder()
            .pixel_size(pic_size)
            .icon_name("image-missing")
            .build();

        let frame = Frame::builder()
            .halign(Align::Center)
            .valign(Align::Center)
            .child(&image)
            .build();

        boxs.append(&frame);

        let label = Label::builder()
            .lines(2)
            .margin_start(10)
            .margin_end(10)
            .width_chars(1)
            .max_width_chars(1)
            .ellipsize(pango::EllipsizeMode::End)
            .wrap(true)
            .margin_top(6)
            .build();

        let label_author = Label::builder()
            .width_chars(1)
            .max_width_chars(1)
            .ellipsize(pango::EllipsizeMode::Middle)
            .wrap(true)
            .margin_top(6)
            .css_classes(
                ["label-album-grid-artist", "dim-label"]
                    .map(String::from)
                    .to_vec(),
            )
            .build();

        boxs.append(&label);
        boxs.append(&label_author);
        (boxs, image, label, label_author)
    }

    pub fn box_update_songlist(
        grid_box: FlowBox,
        song_list: &Vec<SongList>,
        pic_size: i32,
        show_author: bool,
        sender: &Sender<Action>,
    ) {
        for sl in song_list {
            let (boxs, image, label, label_author) = Self::create(pic_size);
            let mut path = crate::path::CACHE.clone();
            path.push(format!("{}-songlist.jpg", sl.id));
            // download cover
            if !path.exists() {
                image.set_from_net(sl.cover_img_url.to_owned(), path, (140, 140), sender);
            } else {
                image.set_from_file(Some(&path));
            }
            image.set_tooltip_text(Some(&sl.name));

            label.set_label(&sl.name);
            if show_author {
                label.set_lines(1);
            }
            label_author.set_label(&sl.author);
            label_author.set_visible(show_author);
            grid_box.insert(&boxs, -1);
        }
    }

    pub fn box_clear(grid: FlowBox) {
        while let Some(child) = grid.last_child() {
            grid.remove(&child);
        }
    }

    fn setup_factory(grid: &GridView, pic_size: i32, show_author: bool) {
        let factory = SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let (boxs, _, label, label_author) = Self::create(pic_size);
            if show_author {
                label.set_lines(1);
            }
            label_author.set_visible(show_author);
            list_item.set_child(Some(&boxs));
        });
        factory.connect_bind(move |_, list_item| {
            let songlist_object = list_item
                .item()
                .unwrap()
                .downcast::<SongListGridItem>()
                .unwrap();

            let frame = list_item.child().unwrap().first_child().unwrap();
            let image = frame.first_child().unwrap();
            let label = frame.next_sibling().unwrap();
            let label_author = label.next_sibling().unwrap();

            songlist_object
                .bind_property("name", &label, "label")
                .sync_create()
                .build();
            songlist_object
                .bind_property("name", &image, "tooltip-text")
                .sync_create()
                .build();
            songlist_object
                .bind_property("name", &label, "label")
                .sync_create()
                .build();
            songlist_object
                .bind_property("author", &label_author, "label")
                .sync_create()
                .build();
            songlist_object
                .property::<Image>("icon")
                .bind_property("paintable", &image, "paintable")
                .sync_create()
                .build();
        });
        grid.set_factory(Some(&factory));
    }

    pub fn view_clear(grid: GridView) {
        grid.set_model(None::<&NoSelection>);
    }

    pub fn view_update_songlist(
        grid: GridView,
        song_list: &[SongList],
        pic_size: i32,
        show_author: bool,
        sender: &Sender<Action>,
    ) {
        Self::setup_factory(&grid, pic_size, show_author);

        let objs: Vec<SongListGridItem> = song_list
            .iter()
            .map(|sl| SongListGridItem::new(sl, sender))
            .collect();

        if let Some(model) = grid.model() {
            let model = model
                .downcast::<NoSelection>()
                .unwrap()
                .model()
                .unwrap()
                .downcast::<gio::ListStore>()
                .unwrap();

            model.extend_from_slice(&objs);
        } else {
            let model = gio::ListStore::new::<SongListGridItem>();
            model.extend_from_slice(&objs);
            let select = NoSelection::new(Some(model));
            grid.set_model(Some(&select));
        }
    }

    pub fn view_item_at_pos(grid: GridView, pos: u32) -> Option<SongListGridItem> {
        grid.model()?.item(pos)?.downcast::<SongListGridItem>().ok()
    }
}

mod imp {

    use super::*;

    #[derive(Default)]
    pub struct SongListGridItem {
        id: Cell<u64>,
        name: RefCell<String>,
        pic_url: RefCell<String>,
        author: RefCell<String>,
        icon: RefCell<Option<Image>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SongListGridItem {
        const NAME: &'static str = "SongListGridItem";
        type Type = super::SongListGridItem;
    }

    impl ObjectImpl for SongListGridItem {
        fn constructed(&self) {
            self.parent_constructed();
            let _obj = self.obj();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecUInt64::builder("id").build(),
                    ParamSpecString::builder("name").build(),
                    ParamSpecString::builder("pic-url").build(),
                    ParamSpecString::builder("author").build(),
                    ParamSpecObject::builder::<Image>("icon").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "id" => {
                    let val = value.get().unwrap();
                    self.id.replace(val);
                }
                "name" => {
                    let val = value.get().unwrap();
                    self.name.replace(val);
                }
                "pic-url" => {
                    let val = value.get().unwrap();
                    self.pic_url.replace(val);
                }
                "author" => {
                    let val = value.get().unwrap();
                    self.author.replace(val);
                }
                "icon" => {
                    let val = value.get().unwrap();
                    self.icon.replace(val);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "id" => self.id.get().to_value(),
                "name" => self.name.borrow().to_value(),
                "pic-url" => self.pic_url.borrow().to_value(),
                "author" => self.author.borrow().to_value(),
                "icon" => self.icon.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}
