//
// local_music_page.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//
// 「本地音乐」独立页面：全部 / 收藏 / 喜爱推荐 / 最近播放 四个视图，
// 收藏与收听统计来自 `local_stats`，只存本机。
use async_channel::Sender;
use gettextrs::gettext;
use glib::Value;
use gtk::{glib, prelude::*, subclass::prelude::*, CompositeTemplate, *};
use ncm_api::SongInfo;
use once_cell::sync::OnceCell;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use crate::{application::Action, gui::songlist_view::SongListView, local_stats};

/// 页面视图
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocalView {
    #[default]
    All,
    Favorites,
    Recommend,
    Recent,
}

/// 推荐/最近播放最多列这么多首
const LIST_LIMIT: usize = 300;

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
        // 本地歌曲可以收藏（走本地统计）、不能跳专辑
        songs_list.set_property("no-act-like", false);
        songs_list.set_property("no-act-album", true);
        if imp.bound.get() {
            return;
        }
        imp.bound.set(true);

        // 全部播放：把当前视图（含搜索过滤）整体播放
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
                page.refresh_view();
            });

        // 视图切换
        let views: [(LocalView, gtk::ToggleButton); 4] = [
            (LocalView::All, imp.mode_all.get()),
            (LocalView::Favorites, imp.mode_fav.get()),
            (LocalView::Recommend, imp.mode_recommend.get()),
            (LocalView::Recent, imp.mode_recent.get()),
        ];
        for (view, button) in views {
            let page = self.clone();
            button.connect_toggled(move |button| {
                if !button.is_active() {
                    return;
                }
                page.imp().view.set(view);
                page.refresh_view();
            });
        }
    }

    /// 订阅当前播放歌曲变化，让列表里的播放指示跟着走（只连接一次）
    pub fn set_window(&self, window: &crate::window::NeteaseCloudMusicGtk4Window) {
        let imp = self.imp();
        if imp.subscribed.get() {
            return;
        }
        imp.subscribed.set(true);
        let songs_list = imp.songs_list.get().downgrade();
        window.connect_local("current-song-changed", false, move |args| {
            let id = args[1].get::<u64>().unwrap_or(0);
            if let Some(songs_list) = songs_list.upgrade() {
                songs_list.update_playing_song(id);
            }
            None
        });
        imp.songs_list
            .get()
            .update_playing_song(window.current_song_id());
    }

    /// 传入本地歌曲全集（窗口打开页面 / 重扫 / 收藏变化时调用）
    pub fn set_songs(&self, songs: Vec<SongInfo>) {
        self.imp().songs.replace(songs);
        self.refresh_view();
    }

    /// 只更新缓存不重绘（「全部」视图下收藏切换不需要整表重排）
    pub fn update_cache(&self, songs: Vec<SongInfo>) {
        self.imp().songs.replace(songs);
        if self.imp().view.get() != LocalView::All {
            self.refresh_view();
        }
    }

    /// 按当前视图 + 搜索词重建列表
    fn refresh_view(&self) {
        let imp = self.imp();
        let view = imp.view.get();
        let key = imp.filter_key.borrow().trim().to_lowercase();

        // 1) 先确定这个视图的候选歌曲
        let candidates: Vec<SongInfo> = match view {
            LocalView::All => imp.songs.borrow().clone(),
            LocalView::Favorites => self.pick(local_stats::favorites()),
            LocalView::Recommend => self.pick(local_stats::recommend(LIST_LIMIT)),
            LocalView::Recent => self.pick(local_stats::recent(LIST_LIMIT)),
        };

        // 2) 搜索过滤
        let filtered: Vec<SongInfo> = if key.is_empty() {
            candidates
        } else {
            candidates
                .into_iter()
                .filter(|song| {
                    song.name.to_lowercase().contains(&key)
                        || song.singer.to_lowercase().contains(&key)
                        || song.album.to_lowercase().contains(&key)
                })
                .collect()
        };

        // 3) 收藏状态 + 推荐视图里垃圾桶 = 不再推荐
        let likes: Vec<bool> = filtered
            .iter()
            .map(|song| local_stats::is_fav(song.id))
            .collect();
        let songs_list = imp.songs_list.get();
        songs_list.set_property("block-remove", view == LocalView::Recommend);
        songs_list.cancel_render();
        songs_list.clear_list();
        songs_list.init_new_list(&filtered, &likes);
        imp.filtered.replace(filtered.clone());

        // 4) 空列表提示
        let hint = imp.hint_label.get();
        let text = match view {
            LocalView::All => String::new(),
            LocalView::Favorites => gettext("No favorites yet — tap the star on a local song."),
            LocalView::Recommend => gettext(
                "Nothing to recommend yet. Play some local songs or star your favorites; \
                 use the trash button to drop a bad suggestion.",
            ),
            LocalView::Recent => gettext("No local song has been played yet."),
        };
        if filtered.is_empty() && !text.is_empty() {
            hint.set_label(&text);
            hint.set_visible(true);
        } else {
            hint.set_visible(false);
        }
    }

    /// 把 id 列表变成 SongInfo：优先用播放列表里的，找不到就用统计里记的路径重建
    fn pick(&self, ids: Vec<u64>) -> Vec<SongInfo> {
        let map: HashMap<u64, SongInfo> = self
            .imp()
            .songs
            .borrow()
            .iter()
            .map(|song| (song.id, song.clone()))
            .collect();
        ids.into_iter()
            .filter_map(|id| match map.get(&id) {
                Some(song) => Some(song.clone()),
                None => local_stats::path_of(id).and_then(|path| {
                    let path = PathBuf::from(path);
                    if path.is_file() {
                        crate::local_music::song_info_from_path(&path)
                    } else {
                        None
                    }
                }),
            })
            .collect()
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
        pub mode_all: TemplateChild<ToggleButton>,
        #[template_child]
        pub mode_fav: TemplateChild<ToggleButton>,
        #[template_child]
        pub mode_recommend: TemplateChild<ToggleButton>,
        #[template_child]
        pub mode_recent: TemplateChild<ToggleButton>,
        #[template_child]
        pub hint_label: TemplateChild<Label>,
        #[template_child]
        pub songs_list: TemplateChild<SongListView>,
        // 本地歌曲全集
        pub songs: Rc<RefCell<Vec<SongInfo>>>,
        // 当前视图过滤后的列表（全部播放用）
        pub filtered: Rc<RefCell<Vec<SongInfo>>>,
        pub filter_key: Rc<RefCell<String>>,
        pub view: Cell<LocalView>,
        pub sender: OnceCell<Sender<Action>>,
        // 信号只连接一次
        pub bound: Cell<bool>,
        // 是否已订阅 current-song-changed
        pub subscribed: Cell<bool>,
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
