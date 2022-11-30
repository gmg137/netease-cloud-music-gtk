//
// songlist_row.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, *};

use crate::application::Action;
use glib::{ParamSpec, ParamSpecBoolean, SendWeakRef, Sender, Value};
use ncm_api::{SongInfo, SongList};
use once_cell::sync::{Lazy, OnceCell};
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

glib::wrapper! {
    pub struct SonglistRow(ObjectSubclass<imp::SonglistRow>)
        @extends Widget, ListBoxRow,
        @implements Accessible, Actionable, Buildable, ConstraintTarget;
}

impl SonglistRow {
    pub fn new(sender: Sender<Action>, si: &SongInfo) -> Self {
        let obj: Self = glib::Object::new(&[]);
        let imp = obj.imp();
        if imp.sender.get().is_none() {
            imp.sender.set(sender).unwrap();
        }
        obj.set_from_song_info(si);
        obj
    }

    pub fn set_from_song_info(&self, si: &SongInfo) {
        self.imp().song_info.replace(Some(si.clone()));

        self.set_tooltip_text(Some(&si.name));
        self.set_name(&si.name);
        self.set_singer(&si.singer);
        self.set_album(&si.album);
        self.set_duration(si.duration);

        self.set_activatable(si.copyright.playable());
    }

    pub fn not_ignore_grey(&self) -> bool {
        self.property("not_ignore_grey")
    }

    pub fn get_song_info(&self) -> Option<SongInfo> {
        self.imp().song_info.borrow().as_ref().cloned()
    }

    pub fn switch_image(&self, visible: bool) {
        let imp = self.imp();
        imp.play_icon.set_visible(visible);
    }

    pub fn set_like_button_visible(&self, visible: bool) {
        let imp = self.imp();
        imp.like_button.set_visible(visible);
    }

    pub fn set_album_button_visible(&self, visible: bool) {
        let imp = self.imp();
        imp.album_button.set_visible(visible);
    }

    fn set_name(&self, label: &str) {
        let imp = self.imp();
        imp.title_label.set_label(label);
    }

    fn set_singer(&self, label: &str) {
        let imp = self.imp();
        imp.artist_label.set_label(label);
    }

    fn set_album(&self, label: &str) {
        let imp = self.imp();
        imp.album_label.set_label(label);
    }

    fn set_duration(&self, duration: u64) {
        let imp = self.imp();
        let label = format!("{:0>2}:{:0>2}", duration / 1000 / 60, duration / 1000 % 60);
        imp.duration_label.set_label(&label);
    }
}

#[gtk::template_callbacks]
impl SonglistRow {
    #[template_callback]
    fn on_click(&self) {
        self.emit_activate();
    }

    #[template_callback]
    fn like_button_clicked_cb(&self) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap();
        let si = { imp.song_info.borrow().clone().unwrap() };
        let s_send = SendWeakRef::from(self.downgrade());
        let like = imp.like.get();
        sender
            .send(Action::LikeSong(
                si.id,
                !like,
                Some(Arc::new(move |_| {
                    if let Some(s) = s_send.upgrade() {
                        s.set_property("like", !like);
                    }
                })),
            ))
            .unwrap();
    }

    #[template_callback]
    fn album_button_clicked_cb(&self) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap();
        let si = { imp.song_info.borrow().clone().unwrap() };
        let songlist = SongList {
            id: si.album_id,
            name: si.album,
            cover_img_url: si.pic_url,
            author: String::new(),
        };
        sender.send(Action::ToAlbumPage(songlist)).unwrap();
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/songlist-row.ui")]
    pub struct SonglistRow {
        #[template_child]
        pub play_icon: TemplateChild<Image>,
        #[template_child]
        pub title_label: TemplateChild<Label>,
        #[template_child]
        pub artist_label: TemplateChild<Label>,
        #[template_child]
        pub album_label: TemplateChild<Label>,
        #[template_child]
        pub duration_label: TemplateChild<Label>,
        #[template_child]
        pub like_button: TemplateChild<Button>,
        #[template_child]
        pub album_button: TemplateChild<Button>,

        pub sender: OnceCell<Sender<Action>>,
        pub song_info: RefCell<Option<SongInfo>>,

        pub like: Cell<bool>,
        pub not_ignore_grey: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SonglistRow {
        const NAME: &'static str = "SonglistRow";
        type Type = super::SonglistRow;
        type ParentType = ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SonglistRow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.bind_property("like", &self.like_button.get(), "icon_name")
                .transform_to(|_, v: bool| {
                    Some(
                        (if v {
                            "starred-symbolic"
                        } else {
                            "non-starred-symbolic"
                        })
                        .to_string(),
                    )
                })
                .build();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("like").build(),
                    ParamSpecBoolean::builder("not-ignore-grey").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "like" => {
                    let like = value.get().expect("The value needs to be of type `bool`.");
                    self.like.replace(like);
                }
                "not-ignore-grey" => {
                    let val: bool = value.get().unwrap();
                    self.not_ignore_grey.replace(val);
                }
                n => unimplemented!("{}", n),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "like" => self.like.get().to_value(),
                "not-ignore-grey" => self.not_ignore_grey.get().to_value(),
                n => unimplemented!("{}", n),
            }
        }
    }
    impl WidgetImpl for SonglistRow {}
    impl ListBoxRowImpl for SonglistRow {}
}
