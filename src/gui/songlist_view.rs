//
// songlist_view.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use gio::Settings;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{CompositeTemplate, glib, *};

use crate::{
    application::Action,
    gui::songlist_row::{SongLikeState, SonglistRow},
};
use async_channel::Sender;
use glib::{
    ParamSpec, ParamSpecBoolean, ParamSpecInt, RustClosure, SignalHandlerId, Value, clone,
    subclass::Signal,
};
use ncm_api::SongInfo;
use once_cell::sync::{Lazy, OnceCell};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
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

#[derive(Debug)]
struct SongItem {
    song: SongInfo,
    like: SongLikeState,
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
        let items: Vec<_> = sis
            .iter()
            .zip(likes)
            .map(|(song, like)| {
                glib::BoxedAnyObject::new(SongItem {
                    song: song.clone(),
                    like: SongLikeState::new(*like),
                })
            })
            .collect();
        let model = self.imp().model.get().unwrap();
        model.splice(model.n_items(), 0, &items);
    }

    pub fn get_songinfo_list(&self) -> Vec<SongInfo> {
        let model = self.imp().model.get().unwrap();
        (0..model.n_items())
            .map(|i| {
                model
                    .item(i)
                    .unwrap()
                    .downcast::<glib::BoxedAnyObject>()
                    .unwrap()
                    .borrow::<SongItem>()
                    .song
                    .clone()
            })
            .collect()
    }

    pub fn clear_list(&self) {
        self.imp().model.get().unwrap().remove_all();
    }

    pub fn update_playing_song(&self, song_id: u64) {
        let imp = self.imp();
        imp.playing_song.set(song_id);
        for row in imp.rows.borrow().values() {
            row.switch_image(row.get_song_info().is_some_and(|song| song.id == song_id));
        }
    }

    fn setup_list(&self) {
        let imp = self.imp();
        let model = gio::ListStore::new::<glib::BoxedAnyObject>();
        imp.listview
            .set_model(Some(&NoSelection::new(Some(model.clone()))));
        imp.model.set(model).unwrap();
        let factory = SignalListItemFactory::new();
        factory.connect_setup(clone!(
            #[weak(rename_to = view)]
            self,
            move |_, item| {
                let item = item.downcast_ref::<ListItem>().unwrap();
                let row = SonglistRow::new(view.imp().sender.get().unwrap().clone());
                row.connect_activate(clone!(
                    #[weak]
                    view,
                    move |row| {
                        if (row.is_activatable() || row.not_ignore_grey())
                            && let Some(song) = row.get_song_info()
                        {
                            view.update_playing_song(song.id);
                            view.imp()
                                .sender
                                .get()
                                .unwrap()
                                .send_blocking(Action::AddPlay(song))
                                .unwrap();
                            view.emit_row_activated(row);
                        }
                    }
                ));
                view.imp()
                    .settings
                    .get()
                    .unwrap()
                    .bind("not-ignore-grey", &row, "not-ignore-grey")
                    .get_only()
                    .build();
                item.set_child(Some(&row));
            }
        ));
        factory.connect_bind(clone!(
            #[weak(rename_to = view)]
            self,
            move |_, item| {
                let item = item.downcast_ref::<ListItem>().unwrap();
                let row = item.child().unwrap().downcast::<SonglistRow>().unwrap();
                let object = item
                    .item()
                    .unwrap()
                    .downcast::<glib::BoxedAnyObject>()
                    .unwrap();
                let data = object.borrow::<SongItem>();
                row.set_from_song_info(&data.song);
                row.bind_like_state(data.like.clone());
                row.set_like_button_visible(!view.property::<bool>("no-act-like"));
                row.set_album_button_visible(!view.property::<bool>("no-act-album"));
                row.set_remove_button_visible(!view.property::<bool>("no-act-remove"));
                row.switch_image(data.song.id == view.imp().playing_song.get());
                view.imp().rows.borrow_mut().insert(item.position(), row);
            }
        ));
        factory.connect_unbind(clone!(
            #[weak(rename_to = view)]
            self,
            move |_, item| {
                let item = item.downcast_ref::<ListItem>().unwrap();
                let row = item.child().unwrap().downcast::<SonglistRow>().unwrap();
                view.imp()
                    .rows
                    .borrow_mut()
                    .retain(|_, bound| bound != &row);
            }
        ));
        imp.listview.set_factory(Some(&factory));
        imp.listview.connect_activate(clone!(
            #[weak(rename_to = view)]
            self,
            move |_, position| {
                let row = view.imp().rows.borrow().get(&position).cloned();
                if let Some(row) = row {
                    row.emit_activate();
                }
            }
        ));
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
        pub adw_clamp: TemplateChild<adw::ClampScrollable>,
        #[template_child]
        pub listview: TemplateChild<ListView>,
        pub model: OnceCell<gio::ListStore>,
        pub rows: RefCell<HashMap<u32, SonglistRow>>,
        pub playing_song: Cell<u64>,

        pub sender: OnceCell<Sender<Action>>,
        pub settings: OnceCell<Settings>,

        no_act_like: Cell<bool>,
        no_act_album: Cell<bool>,
        no_act_remove: Cell<bool>,
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

            obj.setup_list();
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("row-activated")
                        .param_types([SonglistRow::static_type()])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }

        fn properties() -> &'static [ParamSpec] {
            static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
                vec![
                    ParamSpecBoolean::builder("no-act-like").build(),
                    ParamSpecBoolean::builder("no-act-album").build(),
                    ParamSpecBoolean::builder("no-act-remove").build(),
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
                "no-act-remove" => {
                    let val = value.get().unwrap();
                    self.no_act_remove.replace(val);
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
                "no-act-remove" => self.no_act_remove.get().to_value(),
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

#[cfg(test)]
mod tests {
    use super::{Action, SongInfo, SongItem, SongListView, SonglistRow};
    use gtk::{Window, gio, glib, prelude::*, subclass::prelude::*};
    use std::time::{Duration, Instant};

    fn settle() {
        let context = glib::MainContext::default();
        let until = Instant::now() + Duration::from_millis(350);
        while Instant::now() < until {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    #[ignore = "requires a GTK display and the built gresource/schema"]
    fn virtual_list_keeps_all_songs_with_bounded_widgets() {
        adw::init().unwrap();
        let resource = gio::Resource::load(
            std::env::var("NCM_TEST_RESOURCE")
                .expect("set NCM_TEST_RESOURCE to the built gresource"),
        )
        .unwrap();
        gio::resources_register(&resource);
        let (sender, receiver) = async_channel::unbounded();
        let view = SongListView::new();
        view.set_sender(sender.clone());
        view.set_property("no-act-remove", true);
        let songs: Vec<_> = (1..=10_000)
            .map(|id| SongInfo {
                id,
                name: format!("Song {id}"),
                singer: "Artist".into(),
                album: "Album".into(),
                album_id: 1,
                pic_url: String::new(),
                duration: 180_000,
                song_url: String::new(),
                quality: Default::default(),
                copyright: ncm_api::SongCopyright::Free,
            })
            .collect();
        view.init_new_list(&songs, &vec![false; songs.len()]);
        let window = Window::builder()
            .title("Song list regression test (closes automatically)")
            .default_width(1000)
            .default_height(650)
            .child(&view)
            .build();
        window.present();
        settle();
        let initial_rows = view.imp().rows.borrow().len();
        assert!(
            initial_rows > 0 && initial_rows < 300,
            "bound rows: {initial_rows}"
        );
        assert_eq!(view.get_songinfo_list(), songs);
        let adjustment = view.imp().scroll_win.vadjustment();
        for fraction in [0.2, 0.5, 0.9, 1.0] {
            adjustment.set_value((adjustment.upper() - adjustment.page_size()) * fraction);
            settle();
            assert!(view.imp().rows.borrow().len() < 300);
        }
        let last = view.imp().rows.borrow().get(&9999).unwrap().clone();
        assert_eq!(last.get_song_info().unwrap().id, 10_000);
        view.imp()
            .listview
            .emit_by_name::<()>("activate", &[&9999u32]);
        assert!(matches!(receiver.try_recv().unwrap(), Action::AddPlay(song) if song.id == 10_000));
        assert!(last.imp().play_icon.is_visible());
        last.imp().like_button.emit_clicked();
        let callback = match receiver.try_recv().unwrap() {
            Action::LikeSong(10_000, true, Some(callback)) => callback,
            _ => panic!("wrong song liked"),
        };
        adjustment.set_value(0.0);
        settle();
        let rebound = SonglistRow::new(sender);
        let object = view
            .imp()
            .model
            .get()
            .unwrap()
            .item(9999)
            .unwrap()
            .downcast::<glib::BoxedAnyObject>()
            .unwrap();
        rebound.bind_like_state(object.borrow::<SongItem>().like.clone());
        callback(());
        assert!(rebound.property::<bool>("like"));
        assert!(
            view.imp()
                .rows
                .borrow()
                .values()
                .all(|row| !row.property::<bool>("like"))
        );
        view.update_playing_song(10_000);
        adjustment.set_value(adjustment.upper() - adjustment.page_size());
        settle();
        let last = view.imp().rows.borrow().get(&9999).unwrap().clone();
        assert!(last.property::<bool>("like"));
        assert!(last.imp().play_icon.is_visible());
        eprintln!(
            "10,000 songs: {initial_rows} initially bound rows; {} at end; order, activation, recycled likes and playing indicator verified",
            view.imp().rows.borrow().len()
        );
        view.clear_list();
        assert!(view.get_songinfo_list().is_empty());
        view.init_new_list(&songs[..2], &[true, false]);
        settle();
        assert_eq!(view.get_songinfo_list(), songs[..2]);
        view.init_new_list(&songs[2..4], &[false, true]);
        settle();
        assert_eq!(view.get_songinfo_list(), songs[..4]);
        window.close();
        settle();
    }
}
