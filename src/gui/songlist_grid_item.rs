//
// search_songlist_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use glib::Sender;
use glib::{ParamSpec, ParamSpecObject, ParamSpecString, ParamSpecUInt64, Value};
pub(crate) use gtk::{glib, prelude::*, subclass::prelude::*, *};
use ncm_api::SongList;
use once_cell::sync::Lazy;

use crate::{
    application::Action,
    gui::{NcmImageSource, NcmImageSourceObject, NcmPaintable},
};
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
    pub fn new(sl: &SongList) -> Self {
        let s: Self = glib::Object::builder()
            .property("id", &sl.id)
            .property("name", &sl.name)
            .property("pic-url", &sl.cover_img_url)
            .property("author", &sl.author)
            .property(
                "source",
                &NcmImageSource::SongList(sl.id, sl.cover_img_url.clone()).to_gobj(),
            )
            .build();
        s
    }

    fn create(pic_size: i32) -> (gtk::Box, gtk::Image, gtk::Label, gtk::Label) {
        let boxs = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let paintable = NcmPaintable::new(&boxs.display());

        let image = gtk::Image::builder()
            .pixel_size(pic_size)
            .icon_name("image-missing")
            .paintable(&paintable)
            .build();

        let frame = gtk::Frame::builder()
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .child(&image)
            .build();

        boxs.append(&frame);

        let label = gtk::Label::builder()
            .lines(2)
            .margin_start(20)
            .margin_end(20)
            .width_chars(1)
            .max_width_chars(1)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .wrap(true)
            .margin_top(6)
            .build();

        let label_author = gtk::Label::builder()
            .width_chars(1)
            .max_width_chars(1)
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
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
        grid_box: gtk::FlowBox,
        song_list: &Vec<SongList>,
        pic_size: i32,
        show_author: bool,
        _sender: &Sender<Action>,
    ) {
        for sl in song_list {
            let (boxs, image, label, label_author) = Self::create(pic_size);

            image.paintable().unwrap().set_property(
                "source",
                NcmImageSource::SongList(sl.id, sl.cover_img_url.clone()).to_gobj(),
            );

            label.set_label(&sl.name);
            label_author.set_label(&sl.author);
            label_author.set_visible(show_author);

            grid_box.insert(&boxs, -1);
        }
    }

    pub fn box_clear(grid: gtk::FlowBox) {
        while let Some(child) = grid.last_child() {
            grid.remove(&child);
        }
    }

    pub fn view_setup_factory(grid: gtk::GridView, pic_size: i32, show_author: bool) {
        let factory = SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let (boxs, image, label, label_author) = Self::create(pic_size);
            label_author.set_visible(show_author);
            list_item.set_child(Some(&boxs));

            let exp = list_item.property_expression("item");
            exp.chain_property::<SongListGridItem>("name")
                .bind(&label, "label", Widget::NONE);
            exp.chain_property::<SongListGridItem>("author").bind(
                &label_author,
                "label",
                Widget::NONE,
            );
            exp.chain_property::<SongListGridItem>("source").bind(
                &image.paintable().unwrap(),
                "source",
                Widget::NONE,
            );
        });
        // factory.connect_bind(move |_, list_item| {});
        factory.connect_unbind(move |_, list_item| {
            let frame = list_item.child().unwrap().first_child().unwrap();
            let image = frame.first_child().unwrap();
            let label = frame.next_sibling().unwrap();
            let label_author = label.next_sibling().unwrap();
            label.set_property("label", "");
            label_author.set_property("label", "");
            image
                .property::<gdk::Paintable>("paintable")
                .set_property("source", None::<NcmImageSourceObject>);
        });
        grid.set_factory(Some(&factory));
    }

    pub fn view_clear(grid: gtk::GridView) {
        grid.set_model(None::<&NoSelection>);
    }

    pub fn view_update_songlist(
        grid: gtk::GridView,
        song_list: &[SongList],
        _sender: &Sender<Action>,
    ) {
        let objs: Vec<SongListGridItem> = song_list
            .iter()
            .map(|sl| SongListGridItem::new(sl))
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
            let model = gio::ListStore::new(SongListGridItem::static_type());
            model.extend_from_slice(&objs);
            let select = NoSelection::new(Some(&model));
            grid.set_model(Some(&select));
        }
    }

    pub fn view_item_at_pos(grid: gtk::GridView, pos: u32) -> Option<SongListGridItem> {
        grid.model()?.item(pos)?.downcast::<SongListGridItem>().ok()
    }

    pub fn set_texture_from_file(&self, path: &std::path::PathBuf) {
        if let Some(paintable) = gtk::Image::from_file(path).paintable() {
            self.set_property("texture", paintable);
        }
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
        source: RefCell<Option<NcmImageSourceObject>>,
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
                    ParamSpecObject::builder::<NcmImageSourceObject>("source").build(),
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
                "source" => {
                    let val = value.get().unwrap();
                    self.source.replace(val);
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
                "source" => self.source.borrow().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}
