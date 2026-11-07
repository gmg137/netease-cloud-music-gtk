//
// songlist_row.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
use crate::{application::Action, utils::setup_ellipsis_tooltip};
use async_channel::Sender;
use gettextrs::gettext;
use glib::{subclass::Signal, ParamSpec, ParamSpecBoolean, SendWeakRef, SignalHandlerId, Value};
use gtk::{CompositeTemplate, glib, prelude::*, subclass::prelude::*, *};
use ncm_api::{SongInfo, SongList};
use once_cell::sync::{Lazy, OnceCell};
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

glib::wrapper! {
    pub struct SonglistRow(ObjectSubclass<imp::SonglistRow>)
        @extends Widget, Box,
        @implements Accessible, Actionable, Buildable, ConstraintTarget;
}

impl SonglistRow {
    pub fn new(sender: Sender<Action>, si: &SongInfo) -> Self {
        let obj: Self = glib::Object::new();
        let imp = obj.imp();
        if imp.sender.get().is_none() {
            imp.sender.set(sender).unwrap();
        }
        obj.set_from_song_info(si);
        obj.setup_tooltips();
        obj
    }

    pub fn set_from_song_info(&self, si: &SongInfo) {
        self.imp().song_info.replace(Some(si.clone()));

        self.set_name(&si.name);
        self.set_singer(&si.singer);
        self.set_album(&si.album);
        self.set_duration(si.duration);

        self.imp().playable.set(si.copyright.playable());
    }

    /// 是否可播放（不可播放的曲目点击无效，除非用户关了「忽略灰色歌曲」）
    pub fn is_activatable(&self) -> bool {
        self.imp().playable.get()
    }

    pub fn not_ignore_grey(&self) -> bool {
        self.property("not_ignore_grey")
    }

    /// 行被点击（父类换成 Box 后自己提供 activate 信号）
    pub fn emit_activate(&self) {
        self.emit_by_name::<()>("activate", &[]);
    }

    pub fn connect_activate<F: Fn(&Self) + 'static>(&self, f: F) -> SignalHandlerId {
        self.connect_local("activate", false, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
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

    pub fn set_remove_button_visible(&self, visible: bool) {
        let imp = self.imp();
        imp.remove_button.set_visible(visible);
    }

    /// true：垃圾桶按钮改为「不再推荐」
    pub fn set_block_remove(&self, block: bool) {
        self.imp().block_remove.set(block);
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

    fn setup_tooltips(&self) {
        let imp = self.imp();
        for label in [
            imp.title_label.get(),
            imp.artist_label.get(),
            imp.album_label.get(),
        ] {
            setup_ellipsis_tooltip(&label);
        }
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
        let like = imp.like.get();
        // 本地歌曲走本地收藏（不请求账号接口）
        if crate::local_music::is_local_id(si.id) {
            sender
                .send_blocking(Action::ToggleLocalFav(si))
                .unwrap();
            self.set_property("like", !like);
            return;
        }
        let s_send = SendWeakRef::from(self.downgrade());
        sender
            .send_blocking(Action::LikeSong(
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
        if si.album_id != 0 {
            let songlist = SongList {
                id: si.album_id,
                name: si.album,
                cover_img_url: si.pic_url,
                author: String::new(),
                special_type: 0,
            };
            sender.send_blocking(Action::ToAlbumPage(songlist)).unwrap();
        } else {
            sender
                .send_blocking(Action::AddToast(gettext("Album not found!")))
                .unwrap();
        }
    }

    #[template_callback]
    fn remove_button_clicked_cb(&self) {
        let imp = self.imp();
        let sender = imp.sender.get().unwrap();
        let si = { imp.song_info.borrow().clone().unwrap() };
        // 喜爱推荐里垃圾桶 = 「不再推荐」，不动播放列表
        if imp.block_remove.get() {
            sender.send_blocking(Action::BlockRecommend(si)).unwrap();
            return;
        }
        sender
            .send_blocking(Action::RemoveFromPlayList(si))
            .unwrap();
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
        #[template_child]
        pub remove_button: TemplateChild<Button>,

        pub sender: OnceCell<Sender<Action>>,
        pub song_info: RefCell<Option<SongInfo>>,
        // 垃圾桶按钮语义：true = 从喜爱推荐移除（不删播放列表）
        pub block_remove: Cell<bool>,

        pub like: Cell<bool>,
        pub not_ignore_grey: Cell<bool>,
        // 曲目可播放（不再是 ListBoxRow 的 activatable）
        pub playable: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SonglistRow {
        const NAME: &'static str = "SonglistRow";
        type Type = super::SonglistRow;
        type ParentType = Box;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.bind_template_instance_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SonglistRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("activate").build()]
            });
            SIGNALS.as_ref()
        }

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
    impl BoxImpl for SonglistRow {}
}
