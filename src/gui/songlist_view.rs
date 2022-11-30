//
// songlist_view.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gio::Settings;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, *};

use crate::{application::Action, gui::songlist_row::SonglistRow};
use glib::{
    clone, subclass::Signal, ParamSpec, ParamSpecBoolean, ParamSpecInt, RustClosure, Sender,
    SignalHandlerId, Value,
};
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

    fn setup_settings(&self) {
        let settings = Settings::new(crate::APP_ID);

        self.imp()
            .settings
            .set(settings)
            .expect("Could not set `Settings`.");
    }

    pub fn init_new_list(&self, sis: &[SongInfo], likes: &[bool]) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap().to_owned();
        let settings = imp.settings.get().unwrap();

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
            row.connect_activate(clone!(@weak self as s => move |row| {
                if row.is_activatable() || row.not_ignore_grey() {
                    row.switch_image(true);
                    sender.send(Action::AddPlay(si.clone())).unwrap();
                    s.emit_row_activated(row);
                }
            }));

            settings
                .bind("not-ignore-grey", &row, "not-ignore-grey")
                .get_only()
                .build();
            listbox.append(&row);
        });
    }

    pub fn get_songinfo_list(&self) -> Vec<SongInfo> {
        let listbox = self.imp().listbox.get();
        let mut sis: Vec<SongInfo> = vec![];
        if let Some(mut child) = listbox.first_child() {
            loop {
                let row = child.clone().downcast::<SonglistRow>().unwrap();
                sis.push(row.get_song_info().unwrap());

                if let Some(next) = child.next_sibling() {
                    child = next;
                } else {
                    break;
                }
            }
        }
        sis
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

    pub fn emit_row_activated(&self, row: &SonglistRow) {
        self.emit_by_name::<()>("row-activated", &[&row]);
    }

    pub fn connect_row_activated(&self, f: RustClosure) -> SignalHandlerId {
        self.connect_closure("row-activated", false, f)
    }
}

#[gtk::template_callbacks]
impl SongListView {}

mod imp {

    use super::*;

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
        pub settings: OnceCell<Settings>,

        no_act_like: Cell<bool>,
        no_act_album: Cell<bool>,
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

            obj.setup_settings();

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
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("row-activated")
                    .param_types([SonglistRow::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("no-act-like").build(),
                    ParamSpecBoolean::builder("no-act-album").build(),
                    ParamSpecInt::builder("clamp-margin-top").build(),
                    ParamSpecInt::builder("clamp-margin-bottom").build(),
                    ParamSpecInt::builder("clamp-maximum-size").build(),
                    ParamSpecInt::builder("clamp-tightening-threshold").build(),
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
                "clamp-margin-top" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_margin_top(val);
                }
                "clamp-margin-bottom" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_margin_bottom(val);
                }
                "clamp-maximum-size" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_maximum_size(val);
                }
                "clamp-tightening-threshold" => {
                    let val = value.get().unwrap();
                    self.adw_clamp.set_tightening_threshold(val);
                }
                n => unimplemented!("{}", n),
            }
        }

        fn property(&self, _id: usize, pspec: &ParamSpec) -> Value {
            match pspec.name() {
                "no-act-like" => self.no_act_like.get().to_value(),
                "no-act-album" => self.no_act_album.get().to_value(),
                "clamp-margin-top" => self.adw_clamp.margin_top().to_value(),
                "clamp-margin-bottom" => self.adw_clamp.margin_bottom().to_value(),
                "clamp-maximum-size" => self.adw_clamp.maximum_size().to_value(),
                "clamp-tightening-threshold" => self.adw_clamp.tightening_threshold().to_value(),
                n => unimplemented!("{}", n),
            }
        }
    }
    impl WidgetImpl for SongListView {}
    impl BoxImpl for SongListView {}
}
