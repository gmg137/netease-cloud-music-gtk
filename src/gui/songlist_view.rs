//
// songlist_view.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, *};

use crate::{
    application::Action,
    gui::songlist_row::SonglistRow,
    signal::{NcmGSignal, NCM_GSIGNAL},
};
use glib::{closure_local, ParamSpec, ParamSpecBoolean, ParamSpecInt, Sender, Value};
use ncm_api::SongInfo;
use once_cell::sync::{Lazy, OnceCell};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

glib::wrapper! {
    pub struct SongListView(ObjectSubclass<imp::SongListView>)
        @extends Widget, Box,
        @implements Accessible, Actionable, Buildable, ConstraintTarget;
}

impl Default for SongListView {
    fn default() -> Self {
        Self::new()
    }
}

impl SongListView {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_sender(&self, _sender: Sender<Action>) {
        let sender = &self.imp().sender;
        if sender.get().is_none() {
            sender.set(_sender).unwrap();
        }
    }

    pub fn init_new_list(
        &self,
        sis: &Vec<SongInfo>,
        cur_song: Option<u64>,
        is_like_fn: impl Fn(&u64) -> bool,
    ) {
        let sender = self.imp().sender.get().unwrap().to_owned();
        let imp = self.imp();

        let listbox = imp.listbox.get();
        let no_act_album = self.property::<bool>("no-act-album");
        let no_act_like = self.property::<bool>("no-act-like");

        sis.iter().for_each(|si: &SongInfo| {
            let sender = sender.clone();

            let row = SonglistRow::new(sender.clone(), &si);
            row.set_property("like", is_like_fn(&si.id));
            row.set_album_btn_visible(!no_act_album);
            row.set_like_btn_visible(!no_act_like);

            let si = si.to_owned();
            imp.child_map.borrow_mut().insert(si.id, row.to_owned());
            listbox.append(&row);

            row.connect_activate(move |row| {
                row.switch_image(true);
                sender.send(Action::AddPlay(si.to_owned())).unwrap();
            });
        });

        if let Some(song_id) = cur_song {
            self.mark_new_row_playing_with_id(song_id, false);
        }
    }

    pub fn set_songlist_id(&self, id: u64) {
        self.imp().songlist_id.set(id);
    }

    pub fn clear_list(&self) {
        let listbox = self.imp().listbox.get();
        while let Some(child) = listbox.last_child() {
            listbox.remove(&child);
        }
        self.imp().child_map.borrow_mut().clear();
        self.set_songlist_id(0);
    }

    pub fn list_box(&self) -> ListBox {
        self.imp().listbox.get()
    }

    pub fn mark_new_row_playing(&self, index: i32, do_active: bool) {
        let listbox = self.list_box();
        if let Some(row) = listbox.row_at_index(index) {
            let row = row.downcast::<SonglistRow>().unwrap();
            if do_active {
                row.emit_activate();
            } else {
                row.switch_image(true);
            }
            listbox.emit_by_name_with_values("row-activated", &[row.to_value()]);
        }
    }
    pub fn mark_new_row_playing_with_id(&self, song_id: u64, do_active: bool) -> bool {
        let listbox = self.list_box();
        match self.imp().child_map.borrow().get(&song_id) {
            Some(row) => {
                if do_active {
                    row.emit_activate();
                } else {
                    row.switch_image(true);
                }
                listbox.emit_by_name_with_values("row-activated", &[row.to_value()]);
                true
            }
            None => false,
        }
    }

    pub fn mark_song_like(&self, song_id: u64, val: bool) -> bool {
        match self.imp().child_map.borrow().get(&song_id) {
            Some(row) => {
                row.set_property("like", val);
                true
            }
            None => false,
        }
    }

    fn setup_connect(&self) {
        NCM_GSIGNAL
            .get()
            .unwrap()
            .connect_like(closure_local!(@watch self as s =>
                move |_: NcmGSignal, id: u64, val: bool| {
                    s.mark_song_like(id, val);
                }
            ));


        let old_select_row = Rc::new(RefCell::new(-1));
        let old_select_row_1 = old_select_row.to_owned();
        NCM_GSIGNAL
            .get()
            .unwrap()
            .connect_play(closure_local!(@watch self as s =>
                move |_: NcmGSignal, id: u64, album_id: u64, mix_id: u64| {
                    let s_songlist_id = s.imp().songlist_id.get();
                    if s_songlist_id == 0 || s_songlist_id == album_id || s_songlist_id == mix_id {
                        if !s.mark_new_row_playing_with_id(id, false) {
                            if let Some(row) = s.list_box().row_at_index(*old_select_row_1.borrow()) {
                                let row = row.downcast::<SonglistRow>().unwrap();
                                row.switch_image(false);
                            }
                        }
                    }
                }
            ));

        // clear old actived row
        self.imp().listbox.connect_row_activated(move |list, row| {
            let index;
            {
                index = *old_select_row.borrow();
            }
            *old_select_row.borrow_mut() = row.index();
            if index != -1 && index != row.index() {
                if let Some(row) = list.row_at_index(index) {
                    let row = row.downcast::<SonglistRow>().unwrap();
                    row.switch_image(false);
                }
            }
        });
    }
}

#[gtk::template_callbacks]
impl SongListView {}

mod imp {

    use super::*;

    #[derive(Debug, Default)]
    struct Margin {
        top: i32,
        bottom: i32,
        //left: i32,
        //right: i32,
    }

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/songlist-view.ui")]
    pub struct SongListView {
        #[template_child(id = "win")]
        pub scroll_win: TemplateChild<ScrolledWindow>,
        #[template_child(id = "clamp")]
        pub adw_clamp: TemplateChild<adw::Clamp>,
        #[template_child(id = "listbox")]
        pub listbox: TemplateChild<ListBox>,

        pub sender: OnceCell<Sender<Action>>,

        pub songlist_id: Cell<u64>,
        pub child_map: RefCell<HashMap<u64, SonglistRow>>,

        no_act_album: Cell<bool>,
        no_act_like: Cell<bool>,

        content_margin: RefCell<Margin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SongListView {
        const NAME: &'static str = "SongListView";
        type Type = super::SongListView;
        type ParentType = Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_callbacks();
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl SongListView {}

    impl ObjectImpl for SongListView {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let clamp = self.adw_clamp.get();
            obj.bind_property("s-content-margin-top", &clamp, "margin-top")
                .build();
            obj.bind_property("s-content-margin-bottom", &clamp, "margin-bottom")
                .build();

            obj.setup_connect();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("no-act-album").build(),
                    ParamSpecBoolean::builder("no-act-like").build(),
                    ParamSpecInt::builder("s-content-margin-top").build(),
                    ParamSpecInt::builder("s-content-margin-bottom").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "no-act-album" => {
                    let val = value.get().unwrap();
                    self.no_act_album.replace(val);
                }
                "no-act-like" => {
                    let val = value.get().unwrap();
                    self.no_act_like.replace(val);
                }
                "s-content-margin-top" => {
                    let val = value.get().unwrap();
                    self.content_margin.borrow_mut().top = val;
                }
                "s-content-margin-bottom" => {
                    let val = value.get().unwrap();
                    self.content_margin.borrow_mut().bottom = val;
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "no-act-album" => self.no_act_album.get().to_value(),
                "no-act-like" => self.no_act_like.get().to_value(),
                "s-content-margin-top" => self.content_margin.borrow().top.to_value(),
                "s-content-margin-bottom" => self.content_margin.borrow().bottom.to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for SongListView {}
    impl BoxImpl for SongListView {}
}
