//
// songlist_view.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, *};

use crate::{application::Action, gui::songlist_row::SonglistRow};
use glib::{ParamSpec, ParamSpecBoolean, ParamSpecInt, Sender, Value};
use ncm_api::SongInfo;
use once_cell::sync::{Lazy, OnceCell};
use std::{
    cell::{Cell, RefCell},
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

    pub fn init_new_list(&self, sis: &[SongInfo], likes: &[bool], update_lyrics: bool) {
        let sender = self.imp().sender.get().unwrap().to_owned();
        let imp = self.imp();

        let listbox = imp.listbox.get();
        let no_act_like = self.property::<bool>("no-act-like");
        let no_act_album = self.property::<bool>("no-act-album");
        sis.iter().zip(likes.iter()).for_each(|(si, like)| {
            let sender = sender.clone();

            let row = SonglistRow::new(sender.clone(), si);
            row.set_property("like", like);
            row.set_like_button_visible(!no_act_like);
            row.set_album_button_visible(!no_act_album);

            let si = si.clone();
            row.connect_activate(move |row| {
                row.switch_image(true);
                sender.send(Action::AddPlay(si.clone())).unwrap();
                if update_lyrics {
                    sender.send(Action::GetLyrics(si.clone())).unwrap();
                }
            });
            listbox.append(&row);
        });
    }

    pub fn clear_list(&self) {
        let listbox = self.imp().listbox.get();
        while let Some(child) = listbox.last_child() {
            listbox.remove(&child);
        }
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
        #[template_child]
        pub scroll_win: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub adw_clamp: TemplateChild<adw::Clamp>,
        #[template_child]
        pub listbox: TemplateChild<ListBox>,

        pub sender: OnceCell<Sender<Action>>,

        no_act_like: Cell<bool>,
        no_act_album: Cell<bool>,

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

            // clear old actived row
            let old_select_row = Rc::new(RefCell::new(-1));
            self.listbox.connect_row_activated(move |list, row| {
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

            let clamp = self.adw_clamp.get();
            obj.bind_property("s-content-margin-top", &clamp, "margin-top")
                .build();
            obj.bind_property("s-content-margin-bottom", &clamp, "margin-bottom")
                .build();
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("no-act-like").build(),
                    ParamSpecBoolean::builder("no-act-album").build(),
                    ParamSpecInt::builder("s-content-margin-top").build(),
                    ParamSpecInt::builder("s-content-margin-bottom").build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &Value, pspec: &ParamSpec) {
            match pspec.name() {
                "no-act-like" => {
                    let val = value.get().unwrap();
                    self.no_act_like.replace(val);
                }
                "no-act-album" => {
                    let val = value.get().unwrap();
                    self.no_act_album.replace(val);
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
                "no-act-like" => self.no_act_like.get().to_value(),
                "no-act-album" => self.no_act_album.get().to_value(),
                "s-content-margin-top" => self.content_margin.borrow().top.to_value(),
                "s-content-margin-bottom" => self.content_margin.borrow().bottom.to_value(),
                _ => unimplemented!(),
            }
        }
    }
    impl WidgetImpl for SongListView {}
    impl BoxImpl for SongListView {}
}
