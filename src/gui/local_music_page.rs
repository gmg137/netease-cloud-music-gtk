//
// local_music_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
// 「本地音乐」独立页面：列出播放列表里的本地歌曲，支持搜索、全部播放、重新扫描。
use async_channel::Sender;
use glib::Value;
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::SongInfo;
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::{application::Action, gui::songlist_view::SongListView};

glib::wrapper! {
    pub struct LocalMusicPage(ObjectSubclass<imp::LocalMusicPage>)
        @extends adw::Bin, Widget,
        @implements Accessible, Buildable, ConstraintTarget;
}

impl LocalMusicPage {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_sender(&self, sender: Sender<Action>) {
        let imp = self.imp();
        let _ = imp.sender.set(sender.clone());
        let songs_list = imp.songs_list.get();
        songs_list.set_sender(sender.clone());
        // 本地歌曲不能收藏/跳专辑，行内只保留删除按钮
        songs_list.set_property("no-act-like", true);
        songs_list.set_property("no-act-album", true);
        if imp.bound.get() {
            return;
        }
        imp.bound.set(true);

        // 全部播放：把当前过滤后的列表整体播放
        let play_all = imp.play_all_button.get();
        let page = self.clone();
        play_all.connect_clicked(move |_| {
            let songs = page.imp().filtered.borrow().clone();
            if songs.is_empty() {
                return;
            }
            page.imp()
                .sender
                .get()
                .unwrap()
                .send_blocking(Action::AddPlayList(songs, true))
                .unwrap();
        });

        // 重新扫描记住的目录
        let rescan = imp.rescan_button.get();
        let sender_ = imp.sender.get().unwrap().clone();
        rescan.connect_clicked(move |_| {
            sender_.send_blocking(Action::RescanLocal).unwrap();
        });

        // 搜索过滤
        let page = self.clone();
        imp.filter_entry
            .get()
            .connect_search_changed(move |entry| {
                page.imp().filter_key.replace(entry.text().to_string());
                page.apply_filter();
            });
    }

    // 传入本地歌曲全集（窗口打开页面时调用）
    pub fn set_songs(&self, songs: Vec<SongInfo>) {
        self.imp().songs.replace(songs);
        self.apply_filter();
    }

    fn apply_filter(&self) {
        let imp = self.imp();
        let key = imp.filter_key.borrow().trim().to_lowercase();
        let songs = imp.songs.borrow();
        let (filtered, likes): (Vec<SongInfo>, Vec<bool>) = if key.is_empty() {
            (songs.clone(), vec![false; songs.len()])
        } else {
            let hit: Vec<SongInfo> = songs
                .iter()
                .filter(|song| {
                    song.name.to_lowercase().contains(&key)
                        || song.singer.to_lowercase().contains(&key)
                        || song.album.to_lowercase().contains(&key)
                })
                .cloned()
                .collect();
            let count = hit.len();
            (hit, vec![false; count])
        };
        imp.filtered.replace(filtered.clone());
        let songs_list = imp.songs_list.get();
        songs_list.cancel_render();
        songs_list.clear_list();
        songs_list.init_new_list(&filtered, &likes);
    }
}

impl Default for LocalMusicPage {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use once_cell::sync::Lazy;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/gitee/gmg137/NeteaseCloudMusicGtk4/gtk/local-music-page.ui")]
    pub struct LocalMusicPage {
        #[template_child]
        pub filter_entry: TemplateChild<SearchEntry>,
        #[template_child]
        pub play_all_button: TemplateChild<Button>,
        #[template_child]
        pub rescan_button: TemplateChild<Button>,
        #[template_child]
        pub songs_list: TemplateChild<SongListView>,
        // 本地歌曲全集
        pub songs: Rc<RefCell<Vec<SongInfo>>>,
        // 过滤后的列表（全部播放用）
        pub filtered: Rc<RefCell<Vec<SongInfo>>>,
        pub filter_key: Rc<RefCell<String>>,
        pub sender: OnceCell<Sender<Action>>,
        // 信号只连接一次
        pub bound: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LocalMusicPage {
        const NAME: &'static str = "LocalMusicPage";
        type Type = super::LocalMusicPage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LocalMusicPage {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(Vec::new);
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, _value: &Value, pspec: &glib::ParamSpec) {
            pspec.name();
            unimplemented!()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> Value {
            pspec.name();
            unimplemented!()
        }
    }

    impl WidgetImpl for LocalMusicPage {}
    impl adw::subclass::prelude::BinImpl for LocalMusicPage {}
}
