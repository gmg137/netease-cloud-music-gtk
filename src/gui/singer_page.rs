//
// songlist_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gettextrs::gettext;
use glib::{ParamSpec, Sender, Value};
pub(crate) use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::{SingerDetail, SongInfo, SongList};
use once_cell::sync::{Lazy, OnceCell};

use crate::{
    application::Action,
    gui::{NcmImageSource, NcmPaintable, SongListGridItem, SongListView},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

glib::wrapper! {
    pub struct SingerPage(ObjectSubclass<imp::SingerPage>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable,gtk::ConstraintTarget, gtk::Orientable;
}

impl SingerPage {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn init(&self, sender: Sender<Action>) {
        let imp = self.imp();
        imp.sender.set(sender.clone()).unwrap();
        imp.songs_list.set_sender(sender.clone());
        SongListGridItem::view_setup_factory(imp.songlist_grid.get(), 140, false);

        let s_weak = self.clone().downgrade();
        let once = Rc::new(Cell::new(false));
        imp.stack.connect_visible_child_name_notify(move |stack| {
            if once.get() {
                return;
            };
            let name = stack.visible_child_name();
            if name == Some(glib::GString::from("albums")) {
                sender
                    .send(Action::UpdateSingerPageAlbum(glib::SendWeakRef::from(
                        s_weak.clone(),
                    )))
                    .unwrap();
            }
            once.replace(true);
        });
    }

    pub fn singer_id(&self) -> Option<u64> {
        let d = self.imp().detail.borrow();
        d.as_ref().map(|d| d.id)
    }

    pub fn album_offset(&self) -> u16 {
        self.imp().album_offset.get()
    }

    pub fn update_singer_detail(&self, singer: &SingerDetail) {
        let imp = self.imp();
        let avatar = imp.avatar.get();
        avatar.set_text(Some(&singer.name));
        let paintable = NcmPaintable::new(&self.display());
        paintable.connect_texture_loaded(
            glib::closure_local!(@watch avatar  => move |s: NcmPaintable, _:gdk::Texture| {
                avatar.set_custom_image(Some(&s));
            }),
        );
        paintable.set_source(NcmImageSource::Singer(singer.id, singer.cover.clone()));
        imp.avatar_paintable.replace(Some(paintable));

        let title = imp.title_label.get();
        title.set_label(&singer.name);
        imp.detail.replace(Some(singer.clone()));
    }

    pub fn update_songs(&self, songs: &[SongInfo], likes: &[bool]) {
        let imp = self.imp();
        imp.songs_list.init_new_list(&songs, likes);
    }

    pub fn update_albums(&self, song_list: &[SongList]) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap();
        let grid = imp.songlist_grid.get();
        SongListGridItem::view_update_songlist(grid, &song_list, &sender);
        imp.album_offset
            .replace(self.album_offset() + song_list.len() as u16);
    }
}

impl Default for SingerPage {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/singer-page.ui")]
    pub struct SingerPage {
        #[template_child(id = "avatar")]
        pub avatar: TemplateChild<adw::Avatar>,
        #[template_child(id = "title_label")]
        pub title_label: TemplateChild<Label>,
        #[template_child(id = "switcher")]
        pub switcher: TemplateChild<StackSwitcher>,
        #[template_child(id = "stack")]
        pub stack: TemplateChild<Stack>,

        #[template_child(id = "songs_list")]
        pub songs_list: TemplateChild<SongListView>,
        #[template_child(id = "songlist_grid")]
        pub songlist_grid: TemplateChild<gtk::GridView>,

        pub sender: OnceCell<Sender<Action>>,
        pub detail: RefCell<Option<SingerDetail>>,
        pub album_offset: Cell<u16>,

        pub avatar_paintable: RefCell<Option<NcmPaintable>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SingerPage {
        const NAME: &'static str = "SingerPage";
        type Type = super::SingerPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl SingerPage {
        #[template_callback]
        fn album_scrolled_edge_cb(&self, position: PositionType) {
            let album_size = {
                if let Some(detail) = self.detail.borrow().as_ref() {
                    detail.album_size
                } else {
                    0
                }
            };
            if position == gtk::PositionType::Bottom && self.album_offset.get() < album_size as u16
            {
                let sender = self.sender.get().unwrap();
                let s = glib::SendWeakRef::from(self.obj().downgrade());
                sender.send(Action::UpdateSingerPageAlbum(s)).unwrap();
                sender
                    .send(Action::AddToast(gettextrs::gettext(
                        "Loading more content...",
                    )))
                    .unwrap();
            }
        }
        #[template_callback]
        fn album_activate_cb(&self, pos: u32) {
            let sender = self.sender.get().unwrap();
            let grid = self.songlist_grid.get();
            let item = SongListGridItem::view_item_at_pos(grid, pos).unwrap();
            sender.send(Action::ToAlbumPage(item.into())).unwrap();
        }

        #[template_callback]
        fn play_button_clicked_cb(&self) {
            let sender = self.sender.get().unwrap();
            let playlist = self.songs_list.get_songinfo_list();
            if !playlist.is_empty() {
                sender.send(Action::AddPlayList(playlist)).unwrap();
            } else {
                sender
                    .send(Action::AddToast(gettext("This is an empty song list！")))
                    .unwrap();
            }
        }
    }

    impl ObjectImpl for SingerPage {
        fn constructed(&self) {
            self.parent_constructed();
            let _obj = self.obj();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    // ParamSpecBoolean::builder("like").readwrite().build()
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, _value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                n => unimplemented!("{}", n),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                n => unimplemented!("{}", n),
            }
        }
    }
    impl WidgetImpl for SingerPage {}
    impl BoxImpl for SingerPage {}
}
