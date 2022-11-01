//
// songlist_row.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, *};

use crate::application::Action;
use glib::Sender;
use ncm_api::SongInfo;
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};

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
        obj.set_from_song_info(&si);
        obj
    }

    pub fn set_from_song_info(&self, si: &SongInfo) {
        self.imp().song_id.replace(si.id);
        self.imp().album_id.replace(si.album_id);
        self.imp().cover_url.replace(si.pic_url.to_owned());

        self.set_tooltip_text(Some(&si.name));
        self.set_name(&si.name);
        self.set_singer(&si.singer);
        self.set_album(&si.album);
        self.set_duration(&si.duration);
    }

    pub fn switch_image(&self, visible: bool) {
        let imp = self.imp();
        imp.play_icon.set_visible(visible);
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

    fn set_duration(&self, label: &str) {
        let imp = self.imp();
        imp.duration_label.set_label(label);
    }
}

#[gtk::template_callbacks]
impl SonglistRow {
    #[template_callback]
    fn on_click(&self) {
        self.emit_activate();
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

        pub sender: OnceCell<Sender<Action>>,
        pub song_id: Cell<u64>,
        pub album_id: Cell<u64>,
        pub cover_url: RefCell<String>,
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

    impl ObjectImpl for SonglistRow {}
    impl WidgetImpl for SonglistRow {}
    impl ListBoxRowImpl for SonglistRow {}
}
